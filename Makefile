ENDPOINT ?= mainnet.eth.streamingfast.io:443
START_BLOCK ?= 12724620
STOP_BLOCK ?= +1000

.PHONY: build
build:
	cargo build --target wasm32-unknown-unknown --release

.PHONY: run
run: build
	substreams run -e $(ENDPOINT) substreams.yaml map_store_metrics -s $(START_BLOCK) -t $(STOP_BLOCK)

.PHONY: gui
gui: build
	substreams gui -e $(ENDPOINT) substreams.yaml map_store_metrics -s $(START_BLOCK) -t $(STOP_BLOCK)

.PHONY: protogen
protogen:
	substreams protogen ./substreams.yaml --exclude-paths="google,sf/substreams/sink/database,sf/substreams/rpc,sf/substreams/v1"

.PHONY: pack
pack: build
	substreams pack substreams.yaml

.PHONY graph_auth
graph_auth: 
	graph auth --studio e48e546c9d7d775ba6e4d7b439bccd49

.PHONY git_ssh
git_ssh:
	ssh-keygen -t rsa -b 4096 -C "justicessiel@gmail.com" 