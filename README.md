# distributed_MIS

This is an implementation to an randomized distributed algorithm for finding a maximal independent set to any connected graph. 

To build the software, a user needs Rust installed by following https://www.rust-lang.org/tools/install, which should be done by typing one command line

To compuke the software, go to directory which contains Cargo.toml file, then type 
```
cargo build
```

to run the software
```
./run.sh 10 5 3
```
which randomly creates three graph of 10 nodes with max degree 5, output statistics is stored under file n10_d5_r3

to run a specific graph, 
```
./run.sh --graph your_graph 
```

where your_graph is an file containing adjancy list of a graph, every node has id starting from 0. The first column of your input file should contains src node id, dst nodes are seperated by space; no space at the end of each line
