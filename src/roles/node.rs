extern crate rand;
use rand::{Rng, thread_rng};

use std::sync::mpsc::{self, channel, Sender, Receiver, TryRecvError};
use std::collections::{HashMap, HashSet};
use std::{thread, time};
use std::thread::JoinHandle;

 

#[derive(Debug, Clone)]
pub enum Message {
    Request((usize, usize)), //neighbor id, round
    Response(Data),
    JoinedMIS((bool, usize)),
    RemoveNeighbors(HashSet<usize>),
    Start(usize),
    StartRound2,
}

#[derive(Clone, Debug)]
pub enum CentralMessage {
    Step(usize), //id
    Finish(NodeResult), 
    Round1Complete,
    ReconfigComplete(ReconfigResult), //leaving isolated node id
}

#[derive(Copy, Clone, Debug)]
pub struct ReconfigResult {
    pub id: usize,
    pub is_leaving: bool,
}

#[derive(Clone, Debug)]
pub struct NodeResult {
    pub id: usize,
    pub is_in_mis: bool,
    pub nodes_to_remove: Vec<usize>,
}


pub struct Node {
    pub id: usize,
    pub neighbors: HashMap<usize, NeighborContext>,
    pub receiver: Option<Receiver<Message>>,
    pub round: usize,
    pub central_sender: Sender<CentralMessage>,
    pub desire_level: f32,
    pub is_get_marked: bool,
    pub num_response: usize,
    pub is_in_mis: bool,
}

#[derive(Copy, Debug, Clone)]
pub struct Data {
    sender_id: usize,
    round: usize,
    desire_level: f32,
    is_get_marked: bool,
}


pub struct NeighborContext {
    pub round: usize,
    pub sender: Sender<Message>,
    pub is_marked: bool,
    pub desire_level: f32,
    
}

impl NeighborContext {
    pub fn new(sender: Sender<Message>) -> NeighborContext {
        NeighborContext {
            round: 0,
            sender: sender,
            is_marked: false,
            desire_level: 0.5,
        }
    }
    pub fn send(&mut self, message: Message) -> bool {
        match self.sender.send(message) {
            Ok(()) => return true, 
            Err(e) => return false,
        }; 
    }

    pub fn update(&mut self, data: Data) {
        self.round = data.round;
        self.desire_level = data.desire_level;
        self.is_marked = data.is_get_marked;
    }
}


impl Node {
    pub fn new(
        id: usize, 
        central_sender: Sender<CentralMessage>,
    ) -> (Node, Sender<Message>) {
        let (tx, rx) = channel();
        let node = Node {
            id: id,
            neighbors: HashMap::new(),
            receiver: Some(rx),
            round: 0,
            central_sender: central_sender,
            desire_level: 0.5,
            is_get_marked: false,
            num_response: 0,
            is_in_mis: false,
        };
        (node, tx)
    }

    pub fn register_neighbor(&mut self, id: usize, sender: Sender<Message>) {
        match self.neighbors.get(&id) {
            Some(_) => (),
            None => {
                let neighbor_context = NeighborContext::new(sender);
                self.neighbors.insert(id, neighbor_context); 
            },
        }
    }

    pub fn start(mut self) -> JoinHandle<()>{
        std::thread::spawn(move || {
            self.listen() 
        })
    }


    fn clean_neighbors(&mut self) {
        self.neighbors.clear(); 
    }

    fn send_neighbor(&mut self, neighbor_id: usize, message: Message) {
        let neighbor_context = match self.neighbors.get_mut(&neighbor_id) {
            Some(nc) => nc,
            None => panic!("{}.{} unable to find neighbor {}", self.round, self.id, neighbor_id),
        };
        if !neighbor_context.send(message) {
            self.neighbors.remove(&neighbor_id); 
        }
    }

    fn listen(&mut self) {
        let mut num_neighbor_joined = 0;
        let mut any_neighbor_joined: bool = false;
        let peer_receiver = self.receiver.take().unwrap();
        loop {
            

            match peer_receiver.recv() {
                Ok(message) => {
                    match message {
                        Message::Request((neighbor_id, round)) => {
                            //println!("{}.{} Message::Request {} -> {}. request round {}", self.round, self.id, neighbor_id, self.id, round);
                            let data = Data {
                                round: self.round,
                                sender_id: self.id,
                                desire_level: self.desire_level,
                                is_get_marked: self.is_get_marked,
                            };
                            self.send_neighbor(neighbor_id, Message::Response(data));
                        },
                        Message::Response(data) => {
                            //println!("{}.{} Message::Response", self.round, self.id);
                            // check if any neighbor get marked
                            self.num_response += 1; 
                            match self.neighbors.get_mut(&data.sender_id) {
                                Some(neighbor_context) => {
                                    neighbor_context.update(data); 
                                },
                                None => unreachable!(),
                            } 
                            if self.num_response == self.neighbors.len() {
                                //println!("{}.{} Message::Response {} {}", self.round, self.id, self.num_response, self.neighbors.len());
                                self.central_sender.send(CentralMessage::Round1Complete).expect("unable to send Round1Complete"); 
                            }
                        },
                        Message::StartRound2 => {
                            //println!("{}.{} Message::StartRound2", self.round, self.id);
                            // first round exchange phase finishes
                            if !self.is_any_neighbor_marked() && self.is_get_marked {
                                //println!("{} join mis", self.id);
                                self.is_in_mis = true;
                            } else {
                                let effective_degree = self.get_effective_degree();
                                if effective_degree >= 2.0 {
                                    self.desire_level =  self.desire_level / 2.0;
                                } else {
                                    self.desire_level = (2.0*self.desire_level).min(0.5);
                                }
                            }
                            let neighbors: Vec<usize> = self.neighbors.keys().map(|&x| x).collect();
                            for neighbor_id in neighbors {
                                self.send_neighbor(neighbor_id, Message::JoinedMIS((self.is_in_mis, self.id)));
                            }
                        },
                        Message::JoinedMIS((is_neighbor_joined, neighbor_id)) => {
                            //println!("{}.{} Message::JoinedMIS {} {}", self.round, self.id, is_neighbor_joined, neighbor_id);
                            num_neighbor_joined += 1;
                            any_neighbor_joined = any_neighbor_joined | is_neighbor_joined;
                            if num_neighbor_joined == self.neighbors.len() {
                                if any_neighbor_joined | self.is_in_mis {
                                    let result = NodeResult {
                                        id: self.id,
                                        is_in_mis: self.is_in_mis,
                                        nodes_to_remove: vec![self.id],
                                    };
                                    self.central_sender.send(CentralMessage::Finish(result)).expect("unable to send to central");  
                                    //println!("        {} leave network", self.id);
                                } else {
                                    //println!("        {} stay network {:?} ", self.id, self.debug_mark());
                                    self.central_sender.send(CentralMessage::Step(self.id)).expect("central send fail");  
                                }
                                num_neighbor_joined = 0;
                            }
                        },
                        Message::RemoveNeighbors(neighbors_id) => {
                            //println!("{}.{} Message RemoveNeighbors {:?}", self.round,self.id, neighbors_id);
                            for neighbor_id in neighbors_id.iter() {
                                if *neighbor_id == self.id {
                                    self.clean_neighbors();
                                    drop(peer_receiver);
                                    return; 
                                }
                                self.neighbors.remove(neighbor_id);
                            }
                            let mut reconfig_result = ReconfigResult {
                                id: self.id,
                                is_leaving: false,
                            };

                            if self.neighbors.len() == 0{
                                let result = NodeResult {
                                    id: self.id,
                                    is_in_mis: true,
                                    nodes_to_remove: vec![self.id],
                                };
                                //println!("{}. id {} Join MIS", self.round, self.id);
                                reconfig_result.is_leaving = true;
                                self.central_sender.send(CentralMessage::ReconfigComplete(reconfig_result)).expect("central send fail");
                                //println!("{} thread exit", self.id);
                                break;
                            } 
                            
                            self.central_sender.send(CentralMessage::ReconfigComplete(reconfig_result)).expect("unable to send reconfigcomplete");
                        },
                        Message::Start(round) => {
                            //println!("{}.{} Message::Start", round, self.id);
                            // join MIS if there is no neighbors
                            

                            self.num_response = 0;
                            self.round = round;
                            self.is_get_marked = self.decide_if_get_mark();
                            self.request_all_neighbors();

                        },
                    } 
                },
                Err(e) => {
                    //println!("{} thread exit", self.id);
                    break; //println!("node {} try recv error {:?}", self.id, e),
                },
            } 
        }
        drop(peer_receiver);
    }

    fn get_neighbors_id(&self) -> Vec<usize> {
        let mut ids: Vec<usize> = vec![];
        for (id, _) in self.neighbors.iter() {
            ids.push(*id);
        } 
        ids
    }

    fn decide_if_get_mark(&self) -> bool {
        let rand_float = thread_rng().gen_range(0.0, 1.0);
        //println!("float {}", rand_float);
        return rand_float < self.desire_level;
    }

    fn request_all_neighbors(&mut self) {
        let neighbors: Vec<usize> = self.neighbors.keys().map(|&x| x).collect();
        for neighbor_id in neighbors {
            self.send_neighbor(neighbor_id, Message::Request((self.id, self.round)));
        }
    } 

    fn get_effective_degree(&self) -> f32 {
        let mut effective_degree: f32 = 0.0;
        for (_, neighbor_context) in self.neighbors.iter() {
            effective_degree += neighbor_context.desire_level; 
        }    
        effective_degree
    }

    fn debug_mark(&self) -> Vec<(usize, bool)> {
        let mut marked_vec: Vec<(usize, bool)> = vec![];
        for (id, neighbor_context) in self.neighbors.iter() {
            marked_vec.push((*id, neighbor_context.is_marked));
        }
        marked_vec
    }

    fn is_any_neighbor_marked(&self) -> bool {
        for (_, neighbor_context) in self.neighbors.iter() {
            if  neighbor_context.is_marked {
                return true; 
            }
        }
        return false;
    }


}
