#!/bin/bash
# API script

# commands to start p1, p2, p3
# cargo run --release -- -vvv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 | tee p1.out
# cargo run --release -- -vvv --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 | tee p2.out
# cargo run --release -- -vvv --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001 | tee p3.out

# command to start tx_generator and miner
curl http://127.0.0.1:7000/miner/start?lambda=1000000 & \
curl http://127.0.0.1:7001/miner/start?lambda=1000001 & \
curl http://127.0.0.1:7002/miner/start?lambda=1000002

