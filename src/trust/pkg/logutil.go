package pkg

import (
	"log"
)

func Info(msg string, args ...any) {
	log.Printf("ℹ️  "+msg, args...)
}

func Error(msg string, args ...any) {
	log.Printf("❌ "+msg, args...)
}
