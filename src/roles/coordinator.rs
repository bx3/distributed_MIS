use super::node;
use node::{Node, Message, NodeResult, CentralMessage};
use std::sync::mpsc::{TryRecvError, Sender, Receiver, channel};
use std::collections::{HashSet};
use std::sync::{Mutex, Arc};

pub struct Coordinator {
    pub stage: Stage,    
    pub round: usize,
    pub nodes_sender: Vec<Sender<Message>>,
    pub result_list: Vec<NodeResult>,
    pub num_node: usize,
    pub central_receiver: Receiver<CentralMessage>,
}

pub enum Stage {
    Round1,
    Round2,
    Reconfigure,
    Start,
}

impl Coordinator {
    pub fn new(
        nodes_sender: Vec<Sender<Message>>,    
        central_receiver: Receiver<CentralMessage>,
    ) -> Coordinator {
        let num_node = nodes_sender.len();
        Coordinator {
            stage: Stage::Start,
            round: 0,
            nodes_sender: nodes_sender,
            result_list: Vec::new(),
            num_node: num_node,
            central_receiver: central_receiver
        } 
    }

    pub fn start_next_round(&mut self) {
        for sender in  self.nodes_sender.iter_mut() {
            sender.send(Message::Start(self.round)); 
        }
    }

    pub fn inform_nodes(&mut self) {
        for sender in self.nodes_sender.iter_mut() {
            sender.send(Message::StartRound2); 
        }
    }

    pub fn start(&mut self) {
        let mut curr_num_node = self.num_node;
        let mut stage1_num_message = 0;
        let mut stage2_num_message = 0;
        let mut num_reconfig_message = 0;
        let mut num_reconfig_node = 0;
        let mut nodes_to_remove: HashSet<usize> = HashSet::new();
        
        loop {
            if self.result_list.len() == self.num_node {
                break;            
            }

            match self.stage {
                Stage::Start => {
                    println!("Coordinator Start");
                    self.stage = Stage::Round1;
                    self.start_next_round();             
                    
                },
                Stage::Round1 => {
                    if stage1_num_message == curr_num_node {
                        println!("Coordinator Round 1 all collected, {} {} ", stage1_num_message, curr_num_node);
                        self.stage = Stage::Round2;
                        stage1_num_message = 0;
                        self.inform_nodes();
                    } 
                },
                Stage::Round2 => {
                    if stage2_num_message == curr_num_node {
                        println!("Coordinator Round 2 all collected");
                        self.stage = Stage::Reconfigure;
                        stage2_num_message = 0;
                        num_reconfig_node = self.remove_neighbors(&nodes_to_remove);
                        //)
                        println!("num reconfig node {}", num_reconfig_node); 
                        nodes_to_remove.clear();
                    }
                },
                Stage::Reconfigure => {
                    if num_reconfig_message == num_reconfig_node {
                        println!("Coordinator Stage::Reconfigure  all collected. Curr num node {}", curr_num_node);
                        self.stage = Stage::Start; 
                        num_reconfig_message = 0;
                        num_reconfig_node = 0;
                        curr_num_node =curr_num_node - nodes_to_remove.len();
                        self.round += 1;
                    }
                }
            }
    
            // update
            match self.central_receiver.try_recv() {
                Ok(central_message) => {
                    match central_message {
                        CentralMessage::Step(node_id)=> {
                            stage2_num_message += 1;
                        },
                        CentralMessage::Finish(result) => {
                            stage2_num_message += 1;
                            nodes_to_remove.insert(result.id);
                            for id in result.nodes_to_remove.iter() {
                                nodes_to_remove.insert(*id);
                            }
                            self.result_list.push(result);  
                        },
                        CentralMessage::Round1Complete => {
                            stage1_num_message += 1;         
                        },
                        CentralMessage::ReconfigComplete => {
                            num_reconfig_message += 1;
                        }
                    }
                },
                Err(TryRecvError::Empty) =>(),
                Err(e) => {
                    println!("result receiver try receive error {:?}", e); 
                },
            }
        } 
        println!("simulation finishes");
    }

    pub fn get_mis_result(&self) -> HashSet<usize> {
        let mut mis: HashSet<usize> = HashSet::new();
        // post-process results
        for result in self.result_list.iter() {
            if result.is_in_mis {
                mis.insert(result.id); 
            } 
        }
        mis
    } 


    // channel is FIFO
    pub fn remove_neighbors(&mut self, nodes_to_remove: &HashSet<usize>) -> usize {
        let mut num_node_notified = 0;
        for node_id in 0..self.nodes_sender.len() {
            let sender = &mut self.nodes_sender[node_id];
            if nodes_to_remove.len() > 0 {
                if ! nodes_to_remove.contains(&node_id) {
                    sender.send(Message::RemoveNeighbors(nodes_to_remove.clone())); 
                    num_node_notified += 1;
                }
            }
        }
        num_node_notified
    }
}
