//! Centralized user messaging system
//!
//! Provides status and error message handling with fade animations.

use std::time::{Duration, Instant};

// ============================================================================
// Message Types & Priorities
// ============================================================================

/// Types of messages that can be displayed to the user
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    /// General information (blue styling)
    Info,
    /// Error conditions (red styling)
    Error,
}

/// Priority levels for message handling
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Normal priority - status messages
    Normal,
    /// Critical priority - error messages
    Critical,
}

/// A user message with metadata for smart display management
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: Instant,
    pub duration: Duration,
}

impl UserMessage {
    /// Create a new user message
    pub fn new(content: String, message_type: MessageType, priority: MessagePriority) -> Self {
        let duration = match priority {
            MessagePriority::Normal => Duration::from_secs(3),
            MessagePriority::Critical => Duration::from_secs(8),
        };

        Self { content, message_type, timestamp: Instant::now(), duration }
    }

    /// Check if this message has expired
    pub fn is_expired(&self) -> bool {
        // Allow extra time for fade animation (3 second display + 2.1 second fade)
        self.timestamp.elapsed() > (self.duration + std::time::Duration::from_millis(2100))
    }

    /// Check if this message should start fading
    pub fn should_fade(&self) -> bool {
        self.timestamp.elapsed() > self.duration
    }

    /// Get the age of this message in seconds
    pub fn age_seconds(&self) -> f32 {
        self.timestamp.elapsed().as_secs_f32()
    }
}

// ============================================================================
// Message Manager
// ============================================================================

/// Manages user messages with display logic
pub struct MessageManager {
    current_message: Option<UserMessage>,
    message_queue: Vec<UserMessage>,
}

impl MessageManager {
    /// Create a new message manager
    pub fn new() -> Self {
        Self { current_message: None, message_queue: Vec::new() }
    }

    /// Add a new message, overriding current message instantly
    pub fn add_message(&mut self, message: UserMessage) {
        // New messages always override current messages for instant display
        // Save current message to queue only if it hasn't started fading yet
        if let Some(current) = &self.current_message {
            if current.age_seconds() < 3.0 && !current.is_expired() {
                self.message_queue.push(current.clone());
            }
        }

        // Set new message immediately
        self.current_message = Some(message);
        self.cleanup_queue();
    }

    /// Get the current message that should be displayed
    pub fn get_current_message(&self) -> Option<&UserMessage> {
        if let Some(current) = &self.current_message {
            if current.is_expired() {
                None
            } else {
                Some(current)
            }
        } else {
            None
        }
    }

    /// Update the message manager (call this each frame)
    pub fn update(&mut self) {
        if let Some(current) = &self.current_message {
            if current.is_expired() {
                self.current_message = None;
                self.promote_next_message();
            }
        } else {
            self.promote_next_message();
        }
    }

    /// Promote the next message from queue
    fn promote_next_message(&mut self) {
        if let Some(next) = self.message_queue.pop() {
            if !next.is_expired() {
                self.current_message = Some(next);
            } else {
                self.promote_next_message();
            }
        }
    }

    /// Remove expired messages from queue
    fn cleanup_queue(&mut self) {
        self.message_queue.retain(|m| !m.is_expired());

        // Limit queue size
        if self.message_queue.len() > 10 {
            self.message_queue.truncate(10);
        }
    }
}

impl Default for MessageManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a status message
pub fn status_message(content: impl Into<String>) -> UserMessage {
    UserMessage::new(content.into(), MessageType::Info, MessagePriority::Normal)
}

/// Create an error message
pub fn error_message(content: impl Into<String>) -> UserMessage {
    UserMessage::new(content.into(), MessageType::Error, MessagePriority::Critical)
}
