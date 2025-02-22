substream info <spkg github url >

substream info https::/github.com/Graph-BuildersDAO/uniswap-pricing-susbtream/releases/download/v0.1.3/uniswap-pricing-v0.1.3.spkg


  chainlink_prices: https://github.com/Graph-BuildersDAO/substreams/releases/download/chainlink-prices-v1.0.2/chainlink-price-substream-v1.0.2.spkg
  uniswap_prices: https://github.com/Graph-BuildersDAO/uniswap-pricing-substream/releases/download/v0.1.3/uniswap-pricing-v0.1.3.spkg

this would show  the modules in the substream


.PHONY graph_auth
graph_auth: 
	graph auth --studio e48e546c9d7d775ba6e4d7b439bccd49

.PHONY git_ssh
git_ssh:
	ssh-keygen -t rsa -b 4096 -C "justicessiel@gmail.com"  && ssh-add ~/ ssh







  fn tornado_event_mapper(block: eth::Block, chainlink_prices: StoreGetBigDecimal) -> Result<Option<TornadoEvents>, Error> {
