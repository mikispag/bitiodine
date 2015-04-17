all:
	g++ bitiodine_server.cpp -O3 -o bitiodine_server -lemon -lhiredis -lsqlite3 -std=c++11 -pthread -Wl,--no-as-needed
debug:
	g++ bitiodine_server.cpp -g -O3 -o bitiodine_server -lemon -lhiredis -lsqlite3 -std=c++11 -pthread -Wl,--no-as-needed

