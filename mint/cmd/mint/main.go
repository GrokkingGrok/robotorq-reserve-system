package main

import (
    "log"
    "net"
    "net/http"

    "google.golang.org/grpc"
)

func main() {
    // gRPC
    lis, _ := net.Listen("tcp", ":50051")
    s := grpc.NewServer()
    go s.Serve(lis)

    // Health
    http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
        w.Write([]byte("ok"))
    })

    log.Println("Mint running on :50051")
    http.ListenAndServe(":8080", nil)
}
