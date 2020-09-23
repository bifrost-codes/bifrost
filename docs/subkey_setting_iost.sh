../target/release/subkey insert --key-type acco --suri //Alice --chain dev --base-path /tmp/alice
../target/release/subkey insert --key-type acco --suri //Bob --chain dev --base-path /tmp/bob 

# EOS node address
../target/release/subkey localstorage-set --key IOST_NODE_URL --value http://127.0.0.1:30001/ http://127.0.0.1:1234
../target/release/subkey localstorage-set --key IOST_NODE_URL --value http://127.0.0.1:30001/ http://127.0.0.1:4321

# EOS accounts for Multisignature
../target/release/subkey localstorage-set --key IOST_SECRET_KEY --value xjggJ3TrLXz7qEwrGG3Rc4Fz59imjixhXpViq9W7Ncx http://127.0.0.1:1234 # testa
../target/release/subkey localstorage-set --key IOST_SECRET_KEY --value xjggJ3TrLXz7qEwrGG3Rc4Fz59imjixhXpViq9W7Ncx http://127.0.0.1:4321 # testb
