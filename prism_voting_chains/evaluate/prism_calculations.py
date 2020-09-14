
import numpy as np
from random import expovariate, choice
from math import comb


hashpowers = [0.1, 0.2, 0.3, 0.4]
voter_depth_k = [1, 2, 3, 4]
num_voter_chains = [10, 20, 30, 40]

success_prob = {}

for beta in hashpowers:
    nsamples = 100000
    nblocks = 200

    # Simulate block mining times
    adv_wt = np.zeros((nsamples, nblocks), dtype=np.float64)
    honest_wt = np.zeros((nsamples, nblocks), dtype=np.float64)
    for i in range(1, nsamples):
        for j in range(1, nblocks):
            adv_wt[i, j] = expovariate(beta)
            honest_wt[i, j] = expovariate(1 - beta)
    adv_mtime = np.cumsum(adv_wt, axis=1)
    honest_mtime = np.cumsum(honest_wt, axis=1)

    success_prob[beta] = {}
    for k in range(1, 100):
        simlen = max(2*k, k+100)
        count = 0.0
        for i in range(nsamples):
            if np.any(adv_mtime[i, k+1:simlen] < honest_mtime[i, k+1:simlen]):
                count += 1.0

        p = (count/nsamples)
        if np.isclose(p, 0.0):
            break
        else:
            success_prob[beta][k] = p
            
for beta in hashpowers:
    for prism_k in voter_depth_k:
        # p is rev prob for a single chain
        p = success_prob[beta][prism_k]
        for m in num_voter_chains:
            # calculate epsilon overall rev prob guaranteed by prism
            epsilon = 0
            for i in range((m // 2) + 1, m + 1):
                epsilon += (comb(m, i) * pow(p, i) * pow((1-p), (m - i)))
            
            # find k for which bitcoin can guarantee epsilon rev probability
            bitcoin_k = None
            for k in success_prob[beta]:
                if success_prob[beta][k] <= epsilon:
                    bitcoin_k = k
                    break
                    
            if bitcoin_k is None:
                print('Could not find bitcoin_k for beta: %0.1f, prism_k: %d, p: %s, voter chains: %d, rev_prob: %s' 
                      % (beta, prism_k, p, m, epsilon))
            else:
                print('beta: %0.1f, prism_k: %d, p: %s, voter chains: %d, rev_prob: %s, bitcoin_k: %d'
                      % (beta, prism_k, p, m, epsilon, bitcoin_k))
