<a href="https://www.streamingfast.io/">
	<img width="100%" src="https://github.com/streamingfast/substreams/blob/develop/docs/assets/substreams-banner.png" alt="StreamingFast Substreams Banner" />
</a>

# Substreams

Substreams is a powerful blockchain indexing technology, developed for The Graph Network.

Substreams enables developers to write Rust modules, composing data streams alongside the community, and provides extremely high performance indexing by virtue of parallelization, in a streaming-first fashion.

Substreams has all the benefits of StreamingFast Firehose, like low-cost caching and archiving of blockchain data, high throughput processing, and cursor-based reorgs handling.

## Documentation

Full documentation for installing, running and working with Substreams is available at: https://substreams.streamingfast.io.

## Contributing

**Please first refer to the general
[StreamingFast contribution guide](https://github.com/streamingfast/streamingfast/blob/master/CONTRIBUTING.md)**,
if you wish to contribute to this code base.


## License

[Apache 2.0](LICENSE)


Consider store pruning: See Unsiwap V3 SPS and Curve Finance SPS











Fix the decimals issue when setting deposits/withdrawals as we miss out on the decimal points usdc amounts.
Update the ID of withdrawal so it aligns with deposits (use hash, but also include a field for nullifier hash).
Remove commits related to keys.
Align amounts naming convention.
Add immutable decorator to relevant entities.
Add pruning to enable time travel queries 
Add relayer entity and relate it to withdrawals.
Fix bug related to only one withdrawal being visible in subgraph query.

Use Bytes type for ID fields where appropriate.