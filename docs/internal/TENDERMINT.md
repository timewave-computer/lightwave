# Tendermint Consensus and its implications for this recursive Light Client operator

Some relevant background regarding the differences between Helios and Tendermint consensus.

## Pruning
Tendermint chains like Neutron have [archive nodes](https://snapshot.neutron.org/). Non-archive nodes will prune
blocks making it hard for us to recover in production of the prover goes down for an extended period. The request
for the `trusted block header` will fail if said block was pruned and proving will become impossible.

## Trusted Period
Each chain has a notion for a [trusting period](https://docs.neutron.org/relaying/ibc-relayer/?utm_source=chatgpt.com).
In the case of Neutron this period is set to `14 days` which for us means `T` < ~ `1.2M` blocks. I have verified this
and was able to jump `1M` blocks, but unable to jump `1.3M` blocks.

The reason why this is the case is that a validator set is deemed trusted if and only if `1/3rd` of it does not change 
for the duration trusting period. This is much simpler to implement than Ethereum with the committee rotations, since
we don't need to track the rotations, but instead just have to make sure that we don't try to jump more than `T` blocks
at once (and of course that our trusted block has not been pruned).

For now I set the limit to `100k` blocks, meaning if `HEAD` - `TRUSTED` > `100k`, we will default to `100k`.
This value can be changed in `.env`:

```json
# The maximum amount of blocks that we "jump" when proving a tendermint chain
# This depends on the chain spec
TENDERMINT_EXPIRATION_LIMIT=100000
```

## Robustness
Initially I thought that the Tendermint consensus model is vulnerable to frequent committee changes, but in practice this is 
not the case because nodes can't just opt out all at once... and if that happend to any network it'd be dead anyways.

Overall Tendermint consensus seems more simple and easier to work with than Helios, we just need to make sure we use the 
right `trusting period` for each tendermint chain that we deploy this ZK light client for.