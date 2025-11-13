package natsx

import (
	"encoding/json"
	"log"

	"github.com/nats-io/nats.go"
)

type Client struct {
	Conn *nats.Conn
}

func New(url string) (*Client, error) {
	nc, err := nats.Connect(url)
	if err != nil {
		return nil, err
	}
	return &Client{Conn: nc}, nil
}

func (c *Client) Close() {
	if c.Conn != nil {
		c.Conn.Close()
	}
}

func (c *Client) PublishJSON(subject string, v any) error {
	data, err := json.Marshal(v)
	if err != nil {
		return err
	}
	log.Printf("📤 NATS → %s", subject)
	return c.Conn.Publish(subject, data)
}

// ConnectToNATS establishes a connection to a NATS server
func ConnectToNATS(url string) *nats.Conn {
	nc, err := nats.Connect(url)
	if err != nil {
		log.Fatalf("Failed to connect to NATS at %s: %v", url, err)
	}
	log.Printf("Connected to NATS at %s", url)
	return nc
}

// Subscribe subscribes to a NATS subject with a simple message handler
func Subscribe(nc *nats.Conn, subject string, handler func(msg *nats.Msg)) {
	_, err := nc.Subscribe(subject, handler)
	if err != nil {
		log.Fatalf("Failed to subscribe to subject %s: %v", subject, err)
	}
	log.Printf("Subscribed to subject %s", subject)
}
