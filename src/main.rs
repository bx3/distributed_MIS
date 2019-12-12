mod roles;
use std::thread;
use roles::node::{Node, Message, NodeResult, CentralMessage};
use roles::coordinator::{Coordinator};
use roles::creater::{Creater};
use roles::verifier::{Verifier};

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{TryRecvError, Sender, Receiver, channel};
use std::collections::{HashSet};
use std::sync::{Mutex, Arc};

fn main() {
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Bowen Xue.<bx3@uw.edu>")
        (about: "LOCAL network")
        (@arg graph: -g --graph +takes_value "Sets graph file path, graph is an adjacency list, node id incrementally increases by 1, starting at 0")
        (@arg node: -n --node +takes_value "get number of node")
        (@arg degree: -d --degree +takes_value "a parameter for init attempt to create degree (1-degree), but may be more due to connection from other nodes")
        (@arg run: -r --run +takes_value "get number of run")
    )
    .get_matches();

    let graph_path = matches.value_of("graph");
    let mut input_num_node = matches.value_of("node");
    let mut input_num_degree = matches.value_of("degree");
    let mut input_num_run = matches.value_of("run");
    println!("graph  path {:?}", graph_path);
    println!("num_node {:?}", input_num_node);
    println!("num_degree {:?}", input_num_degree);
    println!("num_run {:?}", input_num_run);

    let mut graph: Vec<Vec<usize>> = vec![];
    let mut num_node = 0;
    let mut num_degree = 0;
    let mut graph_creater = Creater::new();

    if graph_path.is_some() {
        let f = File::open(graph_path.unwrap()).expect("Unable to open file");
        let f = BufReader::new(f);
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
    } else {
        num_node = input_num_node.unwrap().parse::<usize>().unwrap();
        num_degree = input_num_degree.unwrap().parse::<usize>().unwrap(); 
        graph = graph_creater.generate(num_node, num_degree);
    }
   
    
    

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

    let mut coordinator = Coordinator::new(sender_list, central_receiver);

    // start simulation
    while let Some(node) = node_list.pop() {
        node.start(); 
    }

    coordinator.start();
    // wait for result
    
    let mis = coordinator.get_mis_result();
    println!("mis {:?}", mis);
    
    let mut verifier = Verifier::new(&mis, &graph);
    let result = verifier.verify();

    println!("graph {:?}", graph);
    println!("Result {}", result);
}


