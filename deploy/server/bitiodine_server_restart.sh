#!/bin/bash

if [ -f /tmp/bitiodine_update_in_progress ]; then
    exit 1
fi

cd ~/bitiodine/deploy/server/
PIDS=`ps aux | grep bitiodine_server | grep -v SCREEN | grep -v grep | grep -v '.sh' | col |  awk '{ print $2; }'`
if [ "${#PIDS}" -lt 2 ]
then
        echo "Restarting..."
        # Send QUIT command to existing instances
        echo QUIT | nc -i 3 -q 5 127.0.0.1 8888
        # Start BitIodine
        screen -S bitiodine -d -m ~/bitiodine/deploy/server/bitiodine_server
fi
