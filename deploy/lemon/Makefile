all:
	g++ bitiodine_server.cpp -O3 -o bitiodine_server -lemon -lhiredis -std=c++11 -pthread -Wl,--no-as-needed
debug:
	g++ bitiodine_server.cpp -g -O0 -o bitiodine_server -lemon -lhiredis -std=c++11 -pthread -ggdb -Wl,--no-as-needed