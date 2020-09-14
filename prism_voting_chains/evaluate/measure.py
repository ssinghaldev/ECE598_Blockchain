import re
import sys
from collections import defaultdict
import numpy as np


class Node:
    def __init__(self):
        self.delay = []
        self.num_entries = 0

    def add_entry(self, sample):
        self.delay.append(sample)
        self.num_entries += 1

    def print_entry(self):
        print('Delay %d Num entries %d Average %0.2f' % (self.delay, self.num_entries, (self.delay/self.num_entries)))

delay_map = defaultdict(Node)
logfiles = ['p1.out', 'p2.out', 'p3.out']
for logfile in logfiles:
    pattern = re.compile("^Received trans hash")
    recv_time_map = {}
    with open(logfile, 'r') as f:
        for line in f:
            if pattern.match(line):
                separated = line.split()
                tx_hash = separated[3]
                recv_time = int(separated[-1])
                # extract the hash and time 
                recv_time_map[tx_hash] = recv_time 

    pattern = re.compile("^Confirmed trans hash")
    confirm_time_map = {}
    with open(logfile, 'r') as f:
        for line in f:
            if pattern.match(line):
                separated = line.split()
                tx_hash = separated[3]
                confirm_time = int(separated[-1])
                # extract the hash and time 
                confirm_time_map[tx_hash] = confirm_time

    for tx_hash in recv_time_map:
        if tx_hash in confirm_time_map:
            delay = confirm_time_map[tx_hash] - recv_time_map[tx_hash]
            delay_map[tx_hash].add_entry(delay)

print('Total confirmed transactions: %d' % (len(delay_map)))


all_data = []
for tx_hash in delay_map:
    if delay_map[tx_hash].num_entries == len(logfiles):
        all_data.extend(delay_map[tx_hash].delay)

print('Total confirmed transactions in all 3 clients: %d' % (len(all_data)/len(logfiles)))
print('Average %0.2f' % (np.mean(all_data)))
print('Standard deviation %0.2f' % np.std(all_data))

            
