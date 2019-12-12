use std::collections::{HashSet};

pub struct Verifier {
    pub mis: HashSet<usize>,
    pub graph: Vec<Vec<usize>>,
}

impl Verifier {
    pub fn new(
        mis: &HashSet<usize>,
        graph: &Vec<Vec<usize>>,
    ) -> Verifier {
        Verifier {
            mis: mis.clone(),
            graph: graph.clone(),
        } 
    }

    fn build_edge_set(&self) -> HashSet<(usize, usize)> {
        let mut edge_set: HashSet<(usize, usize)> = HashSet::new(); 
        for i in 0..self.graph.len() {
            for j in self.graph[i].iter() {
                edge_set.insert((i,*j)); 
                edge_set.insert((*j,i)); 
            } 
        }
        edge_set
    }

    pub fn verify(&self) -> bool {
        let edge_set = self.build_edge_set();
        for i in 0..self.graph.len() {
            if !self.mis.contains(&i) {
                let mut is_none_contain = true;
                for j in self.graph[i].iter() {
                    if self.mis.contains(j) {
                        is_none_contain = false; 
                    }
                }
                if is_none_contain {
                    return false; 
                }
            } 
        }

        true 
    }
}
