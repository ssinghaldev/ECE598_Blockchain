import numpy as np
import matplotlib.pyplot as plt

plt.rcParams['figure.figsize'] = 7.5, 5
plt.rcParams['font.size'] = 12

bitcoin_data = [49647428.69, 80300860.52, 120483180.5, 258741154.8]
bitcoin_err = [7334430.23, 10821801.06, 16159812.19, 18428343.29]

prism_data = [42619377.34, 74636536.39, 90496269.98, 160728385.4]
prism_err = [14232089.62, 21803927.47, 30226432.52, 55756475.08]

def process_data(arr):
    processed = [round(x/1000000, 2) for x in arr]
    return processed

bitcoin_data, bitcoin_err, prism_data, prism_err = list(map(process_data, [bitcoin_data, bitcoin_err, prism_data, prism_err]))


labels = ['10', '20', '30', '40']
x_pos = np.arange(len(labels))

fig, ax = plt.subplots()
ax.bar(x_pos, prism_data,
       yerr=prism_err,
       align='center',
       alpha=0.5,
       ecolor='black',
       capsize=10,
       width=0.7,
       color='g'
      )
ax.set_xlabel('Number of voter chains')

ax.set_ylabel('Average confirmation latency (in secs)')
ax.set_xticks(x_pos)
ax.set_xticklabels(labels)
ax.set_title('Variation of confirmation latency with number of voter chains')
ax.yaxis.grid(True)

# Save the figure and show
plt.tight_layout()
plt.savefig('voterchains.png')


fig, ax = plt.subplots()
ax.bar(x_pos, 
       bitcoin_data,
       yerr=bitcoin_err,
       align='center',
       alpha=0.5,
       ecolor='black',
       capsize=10,
       width=0.4,
       color='r',
       label='Bitcoin'
      )
ax.bar(x_pos+0.4, prism_data,
       yerr=prism_err,
       align='center',
       alpha=0.5,
       ecolor='black',
       capsize=10,
       width=0.4,
       color='g',
       label='Prism'
      )

ax.set_xlabel('Number of voter chains')

ax.set_ylabel('Average confirmation latency (in secs)')
ax.set_xticks(x_pos)
ax.set_xticklabels(labels)
ax.set_title('Bitcoin vs Prism confirmation latency')
ax.yaxis.grid(True)
ax.legend()
# Save the figure and show
plt.tight_layout()
plt.savefig('bitcoinvsprism.png')

exp3_data = [27.24, 42.62, 49.83]
exp3_err = [10.71, 11.52, 14.68]
labels = ['1', '2', '3']
x_pos = np.arange(len(labels))

fig, ax = plt.subplots()
ax.bar(x_pos, exp3_data,
       yerr=exp3_err,
       align='center',
       alpha=0.5,
       ecolor='black',
       capsize=10,
       width=0.4,
       color='g'
      )
ax.set_xlabel('Voter-depth for confirmation')

ax.set_ylabel('Average confirmation latency (in secs)')
ax.set_xticks(x_pos)
ax.set_xticklabels(labels)
ax.set_title('Variation of confirmation latency with voter depth')
ax.yaxis.grid(True)

# Save the figure and show
plt.tight_layout()
plt.savefig('voterdepths.png')



