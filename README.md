# Tornado Cash Substream
This substream's  aim is to show the Tornado Cash Vault Analytics by way of Tracking Deposits and withdrawal in usdc and eth. 

## Quickstart

Make sure you have the latest versions of the following installed:

- [Rust](https://rustup.rs/)
- [Make](https://formulae.brew.sh/formula/make)
- [graph-cli](https://thegraph.com/docs/en/cookbook/quick-start/#2-install-the-graph-cli)
- [substreams-cli](https://substreams.streamingfast.io/getting-started/installing-the-cli)



### 1. Compile the Project with `make build`

We now need to recompile our WASM binary with the new changes we made to the rust files.

### 2. Pack the spkg with `make package`

We need to bundle the protobuf definitions and the WASM binary into a single file. This is what we will deploy the subgraph.

### 3. Deploy the subgraph with `graph deploy`

Modify the package.json to point to your subgraph.
The deploy script will change if you are deploying to the hosted service or decentralized network, but replace this with the command that is appropriate for your setup.

### 4. Schema

```graphql
type PoolStats @entity  {
  id: Bytes!
  totalDepositsInDollars: BigDecimal!
  totalWithdrawalsInDollars: BigDecimal!
}

type Deposit @entity(immutable: true) {
  id: Bytes! 
  commitment: String!
  blockNumber: BigInt!
  timestamp: BigInt!
  eth_amount: BigDecimal!
  from: String!
  usdc_amount: BigDecimal!
}

type Withdrawal @entity(immutable: true) {
  id: Bytes! 
  nullifier_hash: String!
  to: String!
  relayer: Relayer!
  fee: String!
  blockNumber: BigInt!
  timestamp: BigInt!
  eth_amount: BigDecimal!
  usdc_amount: BigDecimal!
}


type Relayer @entity { 
  id: Bytes!
  withdrawal:[Withdrawal!]! @derivedFrom(field: "relayer")
}


```

### 5. Data Flow

```mermaid
graph TD;
  tornado_event_mapper[map: tornado_event_mapper];
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> tornado_event_mapper;
  chainlink_prices:chainlink_price_store --> tornado_event_mapper;
  store_additive_metrics[store: store_additive_metrics];
  tornado_event_mapper --> store_additive_metrics;
  graph_out[map: graph_out];
  tornado_event_mapper --> graph_out;
  store_additive_metrics --> graph_out;
  ethcommon:all_events[map: ethcommon:all_events];
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> ethcommon:all_events;
  ethcommon:all_calls[map: ethcommon:all_calls];
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> ethcommon:all_calls;
  ethcommon:all_events --> ethcommon:index_events;
  ethcommon:all_calls --> ethcommon:index_calls;
  ethcommon:all_events --> ethcommon:index_events_and_calls;
  ethcommon:all_calls --> ethcommon:index_events_and_calls;
  ethcommon:filtered_events[map: ethcommon:filtered_events];
  ethcommon:filtered_events:params[params] --> ethcommon:filtered_events;
  ethcommon:all_events --> ethcommon:filtered_events;
  ethcommon:filtered_calls[map: ethcommon:filtered_calls];
  ethcommon:filtered_calls:params[params] --> ethcommon:filtered_calls;
  ethcommon:all_calls --> ethcommon:filtered_calls;
  ethcommon:filtered_transactions[map: ethcommon:filtered_transactions];
  ethcommon:filtered_transactions:params[params] --> ethcommon:filtered_transactions;
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> ethcommon:filtered_transactions;
  ethcommon:filtered_events_and_calls[map: ethcommon:filtered_events_and_calls];
  ethcommon:filtered_events_and_calls:params[params] --> ethcommon:filtered_events_and_calls;
  ethcommon:all_events --> ethcommon:filtered_events_and_calls;
  ethcommon:all_calls --> ethcommon:filtered_events_and_calls;
  chainlink_prices:store_confirmed_feeds[store: chainlink_prices:store_confirmed_feeds];
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> chainlink_prices:store_confirmed_feeds;
  chainlink_prices:get_chainlink_answers[map: chainlink_prices:get_chainlink_answers];
  sf.ethereum.type.v2.Block[source: sf.ethereum.type.v2.Block] --> chainlink_prices:get_chainlink_answers;
  chainlink_prices:store_confirmed_feeds --> chainlink_prices:get_chainlink_answers;
  chainlink_prices:chainlink_price_store[store: chainlink_prices:chainlink_price_store];
  chainlink_prices:get_chainlink_answers --> chainlink_prices:chainlink_price_store;
  chainlink_prices:graph_out[map: chainlink_prices:graph_out];
  chainlink_prices:get_chainlink_answers --> chainlink_prices:graph_out;


```
