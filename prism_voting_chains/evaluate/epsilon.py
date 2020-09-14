from math import comb
from math import pow

voter_chains = 30

#beta = 0.3 and k = 2
p = 0.31

rev_prob = 0
for i in range(voter_chains // 2, voter_chains + 1):
    rev_prob += (comb(voter_chains, i) * pow(p, i) * pow((1-p), (voter_chains - i)))

print(rev_prob)