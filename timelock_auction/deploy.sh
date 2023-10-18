#!/bin/bash
#
#############################################################################
#                                                                           #
# This is a utility script to deploy the timelock auction to a node.        #
# Written by: Tony Riemer <driemworks@idealabs.network>                     #
#                                                                           #
#############################################################################

# Default URI value
uri="wss://etf1.idealabs.network:443"

# Parse command-line options
while getopts ":u:" opt; do
  case $opt in
    u)
      uri="$OPTARG"
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
    :)
      echo "Option -$OPTARG requires an argument." >&2
      exit 1
      ;;
  esac
done


echo $uri 

# Shift the options so that the non-option arguments are left
shift $((OPTIND-1))

# navigate to compiled contracts directory
cd ./target/ink/
# Step 2: Upload ERC721 contract and store the code hash
erc721_code_hash=$(cargo contract upload erc721/erc721.wasm --suri //Alice --url $uri -x | grep "Code hash" | awk '{print $3}')

# Check if erc721_code_hash is undefined
if [ -z "$erc721_code_hash" ]; then
  echo "Failed to upload ERC721 contract."
  exit 1
fi

echo "Uploaded ERC721. Code Hash: " $erc721_code_hash
# Step 2: Upload Vickrey Auction contract and store the code hash
vickrey_code_hash=$(cargo contract upload vickrey_auction/vickrey_auction.wasm --suri //Alice --url $uri -x | grep "Code hash" | awk '{print $3}')

# Check if vickrey_code_hash is undefined
if [ -z "$vickrey_code_hash" ]; then
  echo "Failed to upload Vickrey Auction contract."
  exit 1
fi

echo "Uploaded Vickrey Auction. Code Hash: " $vickrey_code_hash
# Step 3: Instantiate the proxy contract with arguments as an array
contract_args='['"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"', '"$vickrey_code_hash"', '"$erc721_code_hash"']'
proxy_contract_address=$(cargo contract instantiate tlock_proxy/tlock_proxy.contract --constructor new --args "$contract_args" --suri //Alice --url $uri -x | grep "Contracts -> instantiated" | awk '{print $6}')


# Check if proxy_contract_address is undefined
if [ -z "$proxy_contract_address" ]; then
  echo "Failed to instantiate the proxy contract."
  exit 1
fi

# Step 4: Store the results in deploy_results.txt
echo "auction_contract_code_hash: $vickrey_code_hash" > deploy_results.txt
echo "erc_721_code_hash: $erc721_code_hash" >> deploy_results.txt
echo "proxy_contract_address: $proxy_contract_address" >> deploy_results.txt
echo "erc721_contract_address: " >> deploy_results.txt
