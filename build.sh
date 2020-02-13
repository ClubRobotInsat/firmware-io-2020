#!/bin/bash

if (test $# -ne 1) then 
	a="primary"
else
	a=$1
fi

echo $a
cargo build --release --features=$a
