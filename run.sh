#!/bin/bash
num_node=$1
num_degree=$2
num_run=$3

name="n${num_node}_d${num_degree}_r${num_run}"
if [ -f $name ] ; then
    rm $name
fi

for (( i=0; i < $num_run; ++i ))
do
    target/debug/distributed_MIS --node $1 --degree $2 --run 1 >> $name
done

