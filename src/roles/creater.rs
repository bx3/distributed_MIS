extern crate rand;
use rand::{Rng, thread_rng};
use std::fs::File;
use std::io::{BufWriter, Write, Result};
use bincode::serialize;

pub struct Creater {
    pub graph: Vec<Vec<usize>>,
}

impl Creater {
    pub fn new() -> Creater {
        Creater {
            graph: Vec::new(), 
        } 
    }

    pub fn generate(&mut self, num_node: usize, num_degree: usize) -> Vec<Vec<usize>> {
        self.graph.clear();
        let mut rng = rand::thread_rng();
        for _ in 0..num_node {
            self.graph.push(Vec::new())
        }

        for i in 0..num_node {
            let num_neighbor = rng.gen_range(1, num_degree);
            while self.graph[i].len() < num_neighbor {
                loop {
                    let n: usize = rng.gen_range(0, num_node);
                    if n != i && self.graph[n].len() < num_degree && !self.graph[i].contains(&n){
                        self.graph[i].push(n);
                        self.graph[n].push(i);
                        break;
                    }
                } 
            }
        } 
        self.graph.clone()
    }

    pub fn get_graph(&self) -> Vec<Vec<usize>>  {
        self.graph.clone() 
    }

    pub fn store_graph(&self, filename: String) -> Result<()> {
        let f = File::create(filename).expect("Unable to open file");
        let mut f = BufWriter::new(f);
        for i in 0..self.graph.len() {
            write!(f, "{} ", i);
            for j in self.graph[i].iter() {
                let ser_j = serialize(j).unwrap();
                write!(f, "{}", j);
                write!(f, " ");
            } 
            write!(f, "\n");
        }
        Ok(())
    }

    pub fn get_max_degree(&self) -> usize {
        let mut max_degree = 0;
        for i in 0..self.graph.len() {
            let neighbor_len = self.graph[i].len();
            if neighbor_len > max_degree {
                max_degree = neighbor_len;
            }
        }
        return max_degree;
    }
}
