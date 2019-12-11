mod node;
use std::thread;
use node::{Node, Message, NodeResult, CentralMessage};

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{TryRecvError, Sender, Receiver, channel};
use std::collections::{HashSet};

fn main() {
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Bowen Xue.<bx3@uw.edu>")
        (about: "LOCAL network")
        (@arg graph: -g --graph +takes_value "Sets graph file path, graph is an adjacency list, node id incrementally increases by 1, starting at 0")
    )
    .get_matches();

    let graph_path = matches.value_of("graph").expect("missing neighbor file");

    let f = File::open(graph_path).expect("Unable to open file");
    let f = BufReader::new(f);

    let mut graph: Vec<Vec<usize>> = vec![];
    let mut num_node = 0;
    for line in f.lines() {
        let line = line.expect("Unable to read line");
        num_node += 1;
        let tokens: Vec<&str> = line.split(' ').collect();
        let src = tokens[0];
        let mut dsts: Vec<usize> = vec![];
        for i in 1..tokens.len() {
            dsts.push(tokens[i].parse::<usize>().unwrap());
        }

        graph.push(dsts);
    }
    //println!("tokens {:?}", graph);

    let (central_sender, central_receiver) = channel();
    let mut node_list: Vec<Node> = vec![];
    let mut sender_list: Vec<Sender<Message>> = vec![];

    // initialize nodes
    for i in 0..num_node {
        let (node, sender_to_node) = Node::new(i, central_sender.clone()); 
        node_list.push(node);
        sender_list.push(sender_to_node);
    }

    // connect nodes
    for i in 0..num_node{
        for n_id in graph[i].iter() {
            node_list[*n_id].register_neighbor(i, sender_list[i].clone());
            node_list[i].register_neighbor(*n_id, sender_list[*n_id].clone());
        }
    }

    // start simulation
    let mut round = 1;
    while let Some(node) = node_list.pop() {
        node.start(); 
    }

    println!("start round {:?}", round);
    start_next_round(round, &mut sender_list);    
    
    // wait for result 
    let mut num_received = 0;
    let mut result_list: Vec<NodeResult> = vec![];
    let mut nodes_to_remove: Vec<usize> = vec![];

    loop {
        if result_list.len() == num_node {
            println!("simulation finishes");
            break;            
        }

        if num_received == num_node {
            println!("Round {} send remove node {:?}", round, nodes_to_remove);
            remove_neighbors(&nodes_to_remove, &mut sender_list);
            num_received = 0;
            round += 1;
            nodes_to_remove.clear();
            println!("start round {:?}", round);
            start_next_round(round, &mut sender_list);           
        }

        match central_receiver.try_recv() {
            Ok(central_message) => {
                num_received += 1;
                match central_message {
                    CentralMessage::Step(node_id)=> {
                        println!("Step round {} node {}", round, node_id);   
                    },
                    CentralMessage::Finish(result) => {
                        result_list.push(result);  
                        nodes_to_remove.push(result.id);
                    },
                }
            },
            Err(TryRecvError::Empty) =>(),
            Err(e) => {
                println!("result receiver try receive error {:?}", e); 
            },
        } 
    }



    let mut mis: HashSet<usize> = HashSet::new();
    // post-process results
    for result in result_list.iter() {
        if result.is_in_mis {
            mis.insert(result.id); 
        } 
    }
    println!("mis {:?}", mis);
}

pub fn start_next_round(round: usize, nodes_sender: &mut Vec<Sender<Message>>) {
    for sender in  nodes_sender.iter_mut() {
        sender.send(Message::NextRound(round)); 
    }
}

pub fn remove_neighbors(nodes_to_remove: &Vec<usize>, nodes_sender: &mut Vec<Sender<Message>>) {
    for sender in  nodes_sender.iter_mut() {
        if nodes_to_remove.len() > 0 {
            sender.send(Message::RemoveNeighbors(nodes_to_remove.clone())); 
        }
    }
}
