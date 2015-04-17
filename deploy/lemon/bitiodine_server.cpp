#include <algorithm>
#include <arpa/inet.h>
#include <cstdint>
#include <cstring>
#include <errno.h>
#include <iostream>
#include <iterator>
#include <lemon/bfs.h>
#include <lemon/lgf_reader.h>
#include <lemon/path.h>
#include <lemon/smart_graph.h>
#include <list>
#include <netdb.h>
#include <sqlite3.h>
#include <sstream>
#include <stdlib.h>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>
#include <unordered_map>
#include <unordered_set>
#include <vector>
#include "csv.h"
#include "redis.h"

using namespace lemon;
using namespace std;

// Allowed characters in a Bitcoin address
#define BITCOIN_CHARSET "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

// Enable Redis?
#define USE_REDIS 0

bool bitcoin_address_quick_valid(string address);
void do_command(char *command_c, int client);
string find_path(string from, string to);
unordered_set<string> find_predecessors(string from);
unordered_set<string> find_successors(string from);
void mainloop(int server_fd);
void print_cluster(int client, int cluster);
void print_cluster_id(int client, string address);
void print_cluster_label(int client, int cluster);
int server_establish_connection(int server_fd);
void server_send(int fd, string data);
int server_start_listen();
void server_thread(int rfd);

// Server constants
const char *PORT = "8888";      // port numbers 1-1024 are probably reserved by your OS
const int MAXLEN = 1024;        // Max length of a message.
const int MAXFD = 128;          // Maximum file descriptors to use. Equals maximum clients.
const int BACKLOG = 256;        // Number of connections that can wait in queue before they be accept()'ed

SmartDigraph g;
SmartDigraph::Node genesis = INVALID;
SmartDigraph::NodeMap<string> address(g);
SmartDigraph::ArcMap<string> tx_hash(g);

unordered_map<string, int> clusters;

vector<string> tokenize(string const &input)
{
    istringstream str(input);
    istream_iterator<string> cur(str), end;
    return vector<string>(cur, end);
}

bool bitcoin_address_quick_valid(string address)
{
    int length = address.length();
    if (address[0] != '1' || length < 27 || length > 34) {
        return false;
    }
    bool contains_invalid = address.find_first_not_of(BITCOIN_CHARSET) != string::npos;
    return !contains_invalid;
}

int main()
{
    try {
        digraphReader(g, "../grapher/tx_graph.lgf").  // read the directed graph into g
        arcMap("tx_hash", tx_hash).                   // read the 'tx_hash' arc map into tx_hash
        nodeMap("label", address).
        node("source", genesis).                      // read 'source' node to genesis
        run();
    } catch (Exception &error) {
        cerr << "Error: " << error.what() << endl;
        return -1;
    }

    cerr << "Graph loaded from 'tx_graph.lgf'." << endl;
    cerr << "Number of nodes: " << countNodes(g) << endl;
    cerr << "Number of arcs: " << countArcs(g) << endl;

    io::CSVReader<2> in("../clusterizer/clusters.csv");
    in.read_header(io::ignore_extra_column, "address", "cluster");
    string address; int cluster;
    while (in.read_row(address, cluster)) {
        clusters.insert(make_pair(address, cluster));
    }

    int server_fd = server_start_listen();

    cerr << "Server started." << endl;

    if (server_fd == -1) {
        cerr << "An error occured. Closing program." << endl;
        return 1;
    }

    mainloop(server_fd);
    return 0;
}

void print_cluster_id(int client, string address)
{
    unordered_map<string, int>::const_iterator got = clusters.find(address);

    if (got == clusters.end()) {
        server_send(client, "500 Address not present in any cluster.\n");
        return;
    } else {
        int cluster = got->second;
        server_send(client, "BEGIN\n");
        server_send(client, cluster + "\n");
        server_send(client, "END\n");
    }
}

void print_cluster_label(int client, int cluster)
{
    sqlite3 *db;
    sqlite3_stmt *stmt;

    if (sqlite3_open("../clusterizer/cluster_labels.sqlite", &db) == SQLITE_OK) {
        sqlite3_prepare_v2(db, "SELECT label FROM cluster_labels WHERE cluster_id = ?", -1, &stmt, NULL );
        if (sqlite3_bind_int(stmt, 1, cluster) != SQLITE_OK) {
            server_send(client, "500 No label.\n");
            return;
        }
        if (sqlite3_step(stmt) != SQLITE_OK) {
            server_send(client, "500 No label.\n");
            return;
        }
        string label = std::string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0)));
        server_send(client, "BEGIN\n");
        server_send(client, label + "\n");
        server_send(client, "END\n");
    }
    else {
        cerr << "Failed to open DB!\n";
        server_send(client, "500 No label.\n");
    }

    sqlite3_finalize(stmt);
    sqlite3_close(db);
}

void print_cluster(int client, int cluster)
{
    server_send(client, "BEGIN\n");

    for (auto &it : clusters) {
        if (it.second == cluster) {
            server_send(client, it.first + "\n");
        }
    }

    server_send(client, "END\n");
}

string find_path(string from, string to)
{
    Path<SmartDigraph> p;
    SmartDigraph::Node s = INVALID, t = INVALID;

    int d;
    ostringstream oss;
    string redis_reply = "";
    redisContext *ctx;

    if (USE_REDIS) {
        ctx = redisConnect("127.0.0.1", 6379);

        if (ctx->err) {
            cerr << "Error: " << ctx->errstr << endl;
        }

        // Check for cached response in Redis
        if (ctx) {
            redisReply *reply = (redisReply *) redisCommand(ctx, "GET %s:%s", from.c_str(), to.c_str());
            redis_reply = getRedisString(reply);
        }

        if (!redis_reply.empty()) {
            cerr << "Returning cached response for " << from << ":" << to << endl;
            return redis_reply;
        }
    }

    for (SmartDigraph::NodeIt n(g); (s == INVALID || t == INVALID) && n != INVALID; ++n) {
        if (s == INVALID && address[n] == from)
            s = n;
        if (t == INVALID && address[n] == to)
            t = n;
    }

    if (s == INVALID || t == INVALID)
        return "";

    bool reached = bfs(g).path(p).dist(d).run(s, t);

    if (reached) {
        oss << d << endl;
    } else {
        cerr << "No paths from " << from << " to " << to << "." << endl;
        return "";
    }

    oss << from;

    for (Path<SmartDigraph>::ArcIt a(p); a != INVALID; ++a) {
        oss << ">" << address[g.target(a)];
    }

    oss << endl;

    bool first = true;
    for (Path<SmartDigraph>::ArcIt a(p); a != INVALID; ++a) {
        if (first) {
            first = false;
        } else {
            oss << ">";
        }
        oss << tx_hash[a];
    }

    oss << endl;
    string path = oss.str();

    if (USE_REDIS) {
        // Cache response in Redis
        redisCommand(ctx, "SETEX %s:%s 14400 %s", from.c_str(), to.c_str(), path.c_str());

        if (ctx) {
            redisDisconnect(ctx);
        }
    }

    return path;
}

unordered_set<string> find_successors(string from)
{
    SmartDigraph::Node s = INVALID;
    unordered_set<string> successors;

    string redis_reply, output;
    redisContext *ctx;

    if (USE_REDIS) {
        ctx = redisConnect("127.0.0.1", 6379);

        if (ctx->err) {
            cerr << "Error: " << ctx->errstr << endl;
        }

        /* Check for cached response in Redis */
        if (ctx) {
            redisReply *reply = (redisReply *) redisCommand(ctx, "GET S_%s", from.c_str());
            redis_reply = getRedisString(reply);
        }

        if (!redis_reply.empty()) {
            cerr << "Returning cached response for S_" << from << endl;
            successors.insert(redis_reply);
            return successors;
        }
    }

    for (SmartDigraph::NodeIt n(g); (s == INVALID) && n != INVALID; ++n) {
        if (address[n] == from)
            s = n;
    }

    for (SmartDigraph::OutArcIt a(g, s); a != INVALID && s != INVALID; ++a) {
        successors.insert(address[g.target(a)]);
    }

    for (auto &it : successors) {
        output += it + ",";
    }

    /* Remove last character (,) */
    try {
        output.pop_back();
    } catch (...) {
        // Do nothing for now.
    }

    if (USE_REDIS) {
        // Cache response in Redis
        redisCommand(ctx, "SETEX S_%s 14400 %s", from.c_str(), output.c_str());

        if (ctx) {
            redisDisconnect(ctx);
        }
    }

    return successors;
}

unordered_set<string> find_predecessors(string from)
{
    SmartDigraph::Node s = INVALID;
    unordered_set<string> predecessors;

    string redis_reply = "";
    string output = "";
    redisContext *ctx;

    if (USE_REDIS) {
        ctx = redisConnect("127.0.0.1", 6379);

        if (ctx->err) {
            cerr << "Error: " << ctx->errstr << endl;
        }

        /* Check for cached response in Redis */
        if (ctx) {
            redisReply *reply = (redisReply *) redisCommand(ctx, "GET P_%s", from.c_str());
            redis_reply = getRedisString(reply);
        }

        if (!redis_reply.empty()) {
            cerr << "Returning cached response for P_" << from << endl;
            predecessors.insert(redis_reply);
            return predecessors;
        }
    }

    for (SmartDigraph::NodeIt n(g); (s == INVALID) && n != INVALID; ++n) {
        if (address[n] == from)
            s = n;
    }

    for (SmartDigraph::InArcIt a(g, s); a != INVALID && s != INVALID; ++a) {
        predecessors.insert(address[g.source(a)]);
    }

    for (auto &it : predecessors) {
        output += it + ",";
    }

    /* Remove last character (,) */
    try {
        output.pop_back();
    } catch (...) {
        // Do nothing for now.
    }

    if (USE_REDIS) {
        // Cache response in Redis
        redisCommand(ctx, "SETEX P_%s 14400 %s", from.c_str(), output.c_str());

        if (ctx) {
            redisDisconnect(ctx);
        }
    }

    return predecessors;
}


int server_start_listen()
{
    struct addrinfo hostinfo, *res;

    int sock_fd;

    int server_fd;
    int ret_bind = 1;

    memset(&hostinfo, 0, sizeof(hostinfo));

    hostinfo.ai_family = AF_INET;
    hostinfo.ai_socktype = SOCK_STREAM;
    hostinfo.ai_flags = AI_PASSIVE;

    getaddrinfo(NULL, PORT, &hostinfo, &res);

    server_fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);

    while (ret_bind != 0) {
        cerr << "Trying to kill already running process..." << endl;
        int ret_system = system("echo QUIT | nc -i 3 -q 5 127.0.0.1 8888");
        if (ret_system) {
            cerr << "system() returned " << ret_system << "." << endl;
        }

        ret_bind = ::bind(server_fd, res->ai_addr, res->ai_addrlen);
        cerr << "bind() returned: " << strerror(errno) << endl;
    }

    int ret_listen = listen(server_fd, BACKLOG);

    if (ret_listen != 0) {
        cerr << "Error on listen()." << endl;
        return -1;
    }

    return server_fd;
}

int server_establish_connection(int server_fd)
{
    char ipstr[INET_ADDRSTRLEN];
    int port;
    int new_fd;
    struct sockaddr_storage remote_info;
    socklen_t addr_size;

    addr_size = sizeof(addr_size);
    new_fd = accept(server_fd, (struct sockaddr *) &remote_info, &addr_size);

    getpeername(new_fd, (struct sockaddr *)&remote_info, &addr_size);

    struct sockaddr_in *s = (struct sockaddr_in *)&remote_info;
    port = ntohs(s->sin_port);
    inet_ntop(AF_INET, &s->sin_addr, ipstr, sizeof ipstr);

    cerr << "Connection accepted from "  << ipstr <<  " using port " << port << endl;
    return new_fd;
}

void server_send(int fd, string data)
{
    send(fd, data.c_str(), strlen(data.c_str()), 0);
}

void server_thread(int rfd)
{
    char buf[MAXLEN];

    // Send welcome message
    server_send(rfd, "200 BitIodine 1.0.0 READY\n");

    for (;;) {
        int buflen = read(rfd, buf, sizeof(buf));
        if (buflen <= 0) {
            cerr << "Client disconnected. Clearing fd " << rfd << endl;
            close(rfd);
            return;
        }

        do_command(buf, rfd);

    }
}

void mainloop(int server_fd)
{
    for (;;) {
        int rfd = server_establish_connection(server_fd);

        if (rfd >= 0) {
            cerr << "Client connected. Using file descriptor " << rfd << endl;
            if (rfd > MAXFD) {
                cerr << "Too many clients are trying to connect." << endl;
                close(rfd);
                continue;
            }

            // Spawn a new thread and detach it immediately
            thread(server_thread, rfd).detach();
        }
    }
}

void do_command(char *command_c, int client)
{
    string command = command_c;
    string output;
    vector<string> tokens;
    try {
        tokens = tokenize(command);
    } catch (...) {
        server_send(client, "404 COMMAND NOT FOUND\n");
        return;
    }

    if (tokens[0] == "SHORTEST_PATH_A2A") {
        if (tokens.size() < 3 || !bitcoin_address_quick_valid(tokens[1]) || !bitcoin_address_quick_valid(tokens[2])) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        output = find_path(tokens[1], tokens[2]);

        if (!output.empty()) {
            server_send(client, "BEGIN\n");
            server_send(client, output);
            server_send(client, "END\n");
        } else {
            server_send(client, "500 No path.\n");
        }
        return;
    } else if (tokens[0] == "SUCCESSORS") {
        if (tokens.size() < 2) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        server_send(client, "BEGIN\n");

        unordered_set<string> successors = find_successors(tokens[1]);

        if (!successors.empty()) {
            for (auto &it : successors) {
                output += it + ",";
            }

            /* Remove last character (,) */
            try {
                output.pop_back();
            } catch (...) {
                // Do nothing for now.
            }

            server_send(client, output + "\n");
            server_send(client, "END\n");
        } else
            server_send(client, "500 No successors.\n");
        return;
    } else if (tokens[0] == "PREDECESSORS") {
        if (tokens.size() < 2 || !bitcoin_address_quick_valid(tokens[1])) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        server_send(client, "BEGIN\n");

        unordered_set<string> predecessors = find_predecessors(tokens[1]);

        if (!predecessors.empty()) {
            for (auto &it : predecessors) {
                output += it + ",";
            }

            /* Remove last character (,) */
            try {
                output.pop_back();
            } catch (...) {
                // Do nothing for now.
            }

            server_send(client, output + "\n");
            server_send(client, "END\n");
        } else
            server_send(client, "500 No predecessors.\n");
        return;
    } else if (tokens[0] == "PRINT_CLUSTER") {
        int cluster;

        if (tokens.size() < 2) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        try {
            print_cluster(client, stoi(tokens[1]));
        } catch (std::invalid_argument e) {
            server_send(client, "500 Arguments error.\n");
        }

        return;
    } else if (tokens[0] == "PRINT_CLUSTER_ID") {
        if (tokens.size() < 2) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        try {
            print_cluster_id(client, tokens[1]);
        } catch (std::invalid_argument e) {
            server_send(client, "500 Arguments error.\n");
        }

        return;
    } else if (tokens[0] == "PRINT_CLUSTER_LABEL") {
        int cluster;

        if (tokens.size() < 2) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        try {
            print_cluster_label(client, stoi(tokens[1]));
        } catch (std::invalid_argument e) {
            server_send(client, "500 Arguments error.\n");
        }

        return;
    } else if (tokens[0] == "PRINT_NEIGHBORS") {
        int cluster;

        if (tokens.size() < 2 || !bitcoin_address_quick_valid(tokens[1])) {
            server_send(client, "500 Arguments error.\n");
            return;
        }

        unordered_map<string, int>::const_iterator got = clusters.find(tokens[1]);

        if (got == clusters.end()) {
            server_send(client, "500 Address not present in any cluster.\n");
            return;
        } else {
            cluster = got->second;
        }

        print_cluster(client, cluster);

        return;
    } else if (tokens[0] == "STATS") {
        server_send(client, "BEGIN\n");
        server_send(client, to_string(countNodes(g)));
        server_send(client, "\n");
        server_send(client, to_string(countArcs(g)));
        server_send(client, "\nEND\n");

        return;
    } else if (tokens[0] == "QUIT") {
        exit(0);
    } else {
        server_send(client, "404 COMMAND NOT FOUND\n");
    }
}

