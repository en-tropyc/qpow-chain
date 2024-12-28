# Run the nodes

## Step 0: Clean up the previous runs
```bash
rm -rf /tmp/node0 /tmp/node1 /tmp/node2
```

## Step 1: Run the first node
```bash
RUST_LOG=round-robin=info ./target/release/minimal-template-node --dev --validator --validator-id 0 --total-validators 3 --base-path /tmp/node0 --public-addr /ip4/127.0.0.1/tcp/30333 --rpc-cors all --name "Validator 0"
```

Check the local node ID of the first node. Look for the line:
🏷  Local node identity is: <node-id>

## Step 2: Run the second node
```bash
RUST_LOG=round-robin=info ./target/release/minimal-template-node --dev --validator --validator-id 1 --total-validators 3 --base-path /tmp/node1 --port 30334 --public-addr /ip4/127.0.0.1/tcp/30334 --rpc-port 9945 --name "Validator 1" --rpc-cors all --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/NODE1-ID
```

## Step 3: Run the third node
```bash
RUST_LOG=round-robin=info ./target/release/minimal-template-node --dev --validator --validator-id 2 --total-validators 3 --base-path /tmp/node2 --port 30335 --public-addr /ip4/127.0.0.1/tcp/30335 --rpc-port 9946 --name "Validator 2" --rpc-cors all --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/NODE1-ID
```
