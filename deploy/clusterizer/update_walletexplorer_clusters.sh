#!/bin/bash

for wallet in `curl -s https://www.walletexplorer.com | egrep -o "/wallet/([^\"]+)" | cut -d/ -f3`; do
	for address in `curl -s "https://www.walletexplorer.com/wallet/$wallet/addresses" | egrep -m 1 -o "/address/([^\"]+)" | cut -d/ -f3`; do
		cluster_id=`echo "PRINT_CLUSTER_ID $address" | nc 127.0.0.1 8888 | sed -n 3p`
		if ! [[ $cluster_id =~ ^[0-9]+$ ]] ; then
		   echo "Error: $cluster_id (in $wallet) is not a number" >&2; continue;
		fi
		echo "First address of $wallet is $address, in cluster ID $cluster_id"
		./cluster_labels.py --set $cluster_id "$wallet"
	done
done
