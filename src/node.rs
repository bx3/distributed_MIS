extern crate rand;
use rand::{Rng, thread_rng};

use std::sync::mpsc::{self, channel, Sender, Receiver, TryRecvError};
use std::collections::{HashMap};
use std::{thread, time};

// operation message are sent from main
pub enum OperationMessage {
    JoinedMIS,
    RemoveNeighbors(Vec<usize>),
    NextRound(usize),
} 

#[derive(Debug, Clone)]
pub enum Message {
    Request((usize, usize)), //neighbor id, round
    Response(Data),

    JoinedMIS,
    RemoveNeighbors(Vec<usize>),
    NextRound(usize),
}

#[derive(Copy, Clone, Debug)]
pub enum CentralMessage {
    Step(usize), //id
    Finish(NodeResult), 
}

#[derive(Copy, Clone, Debug)]
pub struct NodeResult {
    pub id: usize,
    pub is_in_mis: bool,
    //pub nodes_to_remove: Vec<usize>,
}


pub struct Node {
    pub id: usize,
    pub neighbors: HashMap<usize, NeighborContext>,
    pub receiver: Receiver<Message>,
    pub round: usize,
    pub central_sender: Sender<CentralMessage>,
    pub desire_level: f32,
    pub is_get_marked: bool,
    pub num_response: usize,
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
    pub fn send(&mut self, message: Message) {
        self.sender.send(message).expect("neighbor context unable to send response"); 
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
            receiver: rx,
            round: 0,
            central_sender: central_sender,
            desire_level: 0.5,
            is_get_marked: false,
            num_response: 0,
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

    pub fn start(mut self) {
        std::thread::spawn(move || {
            self.listen() 
        });
    }


    fn clean_neighbors(&mut self) {
        self.neighbors.clear(); 
    }

    fn listen(&mut self) {
        loop {
            match self.receiver.recv() {
                Ok(message) => {
                    match message {
                        Message::Request((neighbor_id, round)) => {
                            println!("{}. Message::Request {} -> {}. request round {}", self.round, neighbor_id, self.id, round);
                            loop {
                                if self.round == round {
                                    break; 
                                }
                            }
                            let neighbor_context = self.neighbors.get_mut(&neighbor_id).expect("unable to find neighbor");
                            let data = Data {
                                round: self.round,
                                sender_id: self.id,
                                desire_level: self.desire_level,
                                is_get_marked: self.is_get_marked,
                            };
                            neighbor_context.send(Message::Response(data));
                        },
                        Message::Response(data) => {
                            println!("{}. Message::Response id {}", self.round, self.id);
                            // check if any neighbor get marked
                            self.num_response += 1; 
                            match self.neighbors.get_mut(&data.sender_id) {
                                Some(neighbor_context) => {
                                    neighbor_context.update(data); 
                                },
                                None => unreachable!(),
                            }

                            // first round exchange phase finishes
                            if self.num_response == self.neighbors.len() {
                                if !self.is_any_neighbor_marked() {
                                    // send to central controller join MIS
                                    let result = NodeResult {
                                        id: self.id,
                                        is_in_mis: true,
                                    };
                                    self.central_sender.send(CentralMessage::Finish(result));
                                    // send neighbor to join MIS
                                    //for (_, neighbor_context) in self.neighbors.iter_mut() {
                                    //    neighbor_context.send(Message::JoinedMIS); 
                                    //}
                                    self.clean_neighbors();

                                } else {
                                    // update desire level    
                                    let effective_degree = self.get_effective_degree();
                                    if effective_degree >= 2.0 {
                                        self.desire_level =  self.desire_level / 2.0;
                                    } else {
                                        self.desire_level = (2.0*self.desire_level).min(0.5);
                                    }
                                    self.central_sender.send(CentralMessage::Step(self.id));
                                }
                            }
                        },
                        Message::JoinedMIS => {
                            println!("{}. Message::JoinedMIS id {}", self.round, self.id);
                            self.clean_neighbors();
                            let result = NodeResult {
                                id: self.id,
                                is_in_mis: false,
                            };
                            self.central_sender.send(CentralMessage::Finish(result));
                            // terminate

                            let ten_millis = time::Duration::from_millis(10);
                            thread::sleep(ten_millis); 
                            
                            break;
                        },
                        Message::RemoveNeighbors(neighbors_id) => {
                            println!("{}. Message RemoveNeighbors {:?}", self.round, neighbors_id);
                            for neighbor_id in neighbors_id {
                                if neighbor_id == self.id {
                                    self.clean_neighbors();
                                    return; 
                                }
                                self.neighbors.remove(&neighbor_id);
                            }
                        },
                        Message::NextRound(round) => {
                            println!("{}. Message::NextRound id {}", round, self.id);
                            if round != self.round + 1 {
                                panic!("mismatch round local {} message {}", self.round, round); 
                            }   
                            self.is_get_marked = self.decide_if_get_mark();
                            self.round += 1;
                            self.request_all_neighbors();

                        },
                    } 
                },
                Err(e) => println!("node {} try recv error {:?}", self.id, e),
            } 
        } 
    }

    fn decide_if_get_mark(&self) -> bool {
        let rand_float = thread_rng().gen_range(0.0, 1.0); ;
        return rand_float > self.desire_level;
    }

    fn request_all_neighbors(&mut self) {
        for (_, neighbor_context) in self.neighbors.iter_mut() {
            neighbor_context.send(Message::Request((self.id, self.round))); 
        }
    } 

    fn get_effective_degree(&self) -> f32 {
        let mut effective_degree: f32 = 0.0;
        for (_, neighbor_context) in self.neighbors.iter() {
            effective_degree += neighbor_context.desire_level; 
        }    
        effective_degree
    }

    fn is_any_neighbor_marked(&self) -> bool {
        for (_, neighbor_context) in self.neighbors.iter() {
            if  neighbor_context.is_marked {
                return true; 
            }
        }
        println!("{}. {}. none marked", self.round, self.id);
        return false;
    }


}
