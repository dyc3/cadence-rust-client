//! Workflow channels for coordinating parallel execution.
//!
//! Channels are ephemeral (in-memory only) and are not recorded to workflow history.
//! They enable deterministic parallel coordination patterns like fan-out/fan-in and
//! split-merge workflows while maintaining replay safety.
//!
//! # Example
//! ```rust,ignore
//! let (tx, rx) = ctx.new_channel(10); // buffered channel with capacity 10
//!
//! ctx.spawn(async move {
//!     tx.send(42).await.unwrap();
//! });
//!
//! let value = rx.recv().await; // Some(42)
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// Error returned when trying to send on a closed channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendError<T>(pub T);

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel closed")
    }
}

impl<T: fmt::Debug> std::error::Error for SendError<T> {}

/// Error returned when trying to send on a full channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrySendError<T> {
    Full(T),
    Closed(T),
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrySendError::Full(_) => write!(f, "channel full"),
            TrySendError::Closed(_) => write!(f, "channel closed"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TrySendError<T> {}

/// Error returned when trying to receive from an empty channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Closed,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryRecvError::Empty => write!(f, "channel empty"),
            TryRecvError::Closed => write!(f, "channel closed"),
        }
    }
}

impl std::error::Error for TryRecvError {}

/// Internal channel state shared between Sender and Receiver
struct ChannelInner<T> {
    /// Buffer for storing messages
    buffer: VecDeque<T>,
    /// Channel capacity (0 = unbuffered)
    capacity: usize,
    /// Whether the channel is closed
    closed: bool,
    /// Number of active senders
    sender_count: usize,
    /// Wakers for blocked send operations
    blocked_sends: Vec<Waker>,
    /// Wakers for blocked receive operations
    blocked_recvs: Vec<Waker>,
}

impl<T> ChannelInner<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            capacity,
            closed: false,
            sender_count: 1,
            blocked_sends: Vec::new(),
            blocked_recvs: Vec::new(),
        }
    }

    /// Wake all blocked senders
    fn wake_senders(&mut self) {
        for waker in self.blocked_sends.drain(..) {
            waker.wake();
        }
    }

    /// Wake all blocked receivers
    fn wake_receivers(&mut self) {
        for waker in self.blocked_recvs.drain(..) {
            waker.wake();
        }
    }

    /// Check if we can send (buffer not full or unbuffered with waiting receiver)
    fn can_send(&self) -> bool {
        if self.closed {
            return false;
        }
        if self.capacity == 0 {
            // Unbuffered: can send if there's a waiting receiver
            !self.blocked_recvs.is_empty()
        } else {
            // Buffered: can send if buffer not full
            self.buffer.len() < self.capacity
        }
    }

    /// Check if we can receive (buffer not empty or unbuffered with waiting sender)
    fn can_recv(&self) -> bool {
        if !self.buffer.is_empty() {
            return true;
        }
        if self.capacity == 0 {
            // Unbuffered: can recv if there's a waiting sender
            !self.blocked_sends.is_empty()
        } else {
            false
        }
    }

    /// Check if channel is effectively closed for receivers
    fn is_closed_for_recv(&self) -> bool {
        self.closed && self.buffer.is_empty()
    }
}

/// Sending half of a channel
pub struct Sender<T> {
    inner: Arc<Mutex<ChannelInner<T>>>,
}

impl<T> Sender<T> {
    /// Send a value through the channel.
    ///
    /// This will block if the buffer is full (or if unbuffered, until a receiver is ready).
    /// Returns an error if all receivers have been dropped.
    pub fn send(&self, value: T) -> SendFuture<T> {
        SendFuture {
            sender: self.clone(),
            value: Some(value),
        }
    }

    /// Try to send a value without blocking.
    ///
    /// Returns:
    /// - `Ok(())` if the value was sent
    /// - `Err(TrySendError::Full(value))` if the buffer is full
    /// - `Err(TrySendError::Closed(value))` if all receivers are dropped
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        let mut inner = self.inner.lock().unwrap();

        if inner.closed {
            return Err(TrySendError::Closed(value));
        }

        if !inner.can_send() {
            return Err(TrySendError::Full(value));
        }

        // Send the value
        inner.buffer.push_back(value);

        // Wake a blocked receiver
        if let Some(waker) = inner.blocked_recvs.pop() {
            waker.wake();
        }

        Ok(())
    }

    /// Close the channel from the sender side
    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        inner.wake_receivers();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.inner.lock().unwrap();
        inner.sender_count += 1;
        drop(inner);

        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.sender_count -= 1;

        // If this was the last sender, close the channel
        if inner.sender_count == 0 {
            inner.closed = true;
            inner.wake_receivers();
        }
    }
}

/// Receiving half of a channel
pub struct Receiver<T> {
    inner: Arc<Mutex<ChannelInner<T>>>,
}

impl<T> Receiver<T> {
    /// Receive a value from the channel.
    ///
    /// Returns `None` if the channel is closed and empty.
    /// Blocks if the channel is empty but not closed.
    pub fn recv(&self) -> RecvFuture<T> {
        RecvFuture {
            receiver: self.clone(),
        }
    }

    /// Try to receive a value without blocking.
    ///
    /// Returns:
    /// - `Ok(value)` if a value was received
    /// - `Err(TryRecvError::Empty)` if the channel is empty but not closed
    /// - `Err(TryRecvError::Closed)` if the channel is closed and empty
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(value) = inner.buffer.pop_front() {
            // Wake a blocked sender since space is now available
            if let Some(waker) = inner.blocked_sends.pop() {
                waker.wake();
            }
            return Ok(value);
        }

        if inner.is_closed_for_recv() {
            Err(TryRecvError::Closed)
        } else {
            Err(TryRecvError::Empty)
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Future returned by `Sender::send()`
pub struct SendFuture<T> {
    sender: Sender<T>,
    value: Option<T>,
}

impl<T> Future for SendFuture<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = this.sender.inner.lock().unwrap();

        if inner.closed {
            let value = this.value.take().unwrap();
            return Poll::Ready(Err(SendError(value)));
        }

        if inner.can_send() {
            let value = this.value.take().unwrap();
            inner.buffer.push_back(value);

            // Wake a blocked receiver
            if let Some(waker) = inner.blocked_recvs.pop() {
                waker.wake();
            }

            return Poll::Ready(Ok(()));
        }

        // Register waker for when space becomes available
        inner.blocked_sends.push(cx.waker().clone());
        Poll::Pending
    }
}

/// Future returned by `Receiver::recv()`
pub struct RecvFuture<T> {
    receiver: Receiver<T>,
}

impl<T> Future for RecvFuture<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.receiver.inner.lock().unwrap();

        // Try to receive from buffer
        if let Some(value) = inner.buffer.pop_front() {
            // Wake a blocked sender since space is now available
            if let Some(waker) = inner.blocked_sends.pop() {
                waker.wake();
            }
            return Poll::Ready(Some(value));
        }

        // Channel is closed and empty
        if inner.is_closed_for_recv() {
            return Poll::Ready(None);
        }

        // Register waker for when a value arrives
        inner.blocked_recvs.push(cx.waker().clone());
        Poll::Pending
    }
}

/// Create a new channel with the specified buffer size.
///
/// # Arguments
/// * `buffer_size` - Capacity of the buffer. Use 0 for unbuffered channel.
///
/// # Returns
/// A tuple of (Sender, Receiver)
pub fn channel<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Mutex::new(ChannelInner::new(buffer_size)));

    let sender = Sender {
        inner: inner.clone(),
    };

    let receiver = Receiver { inner };

    (sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffered_channel_send_recv() {
        let (tx, rx) = channel(2);

        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();

        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(rx.recv().await, Some(2));
    }

    #[tokio::test]
    async fn test_channel_closes_when_sender_dropped() {
        let (tx, rx) = channel::<i32>(1);

        drop(tx);

        assert_eq!(rx.recv().await, None);
    }

    #[tokio::test]
    async fn test_try_send_full() {
        let (tx, _rx) = channel(1);

        tx.try_send(1).unwrap();
        assert!(matches!(tx.try_send(2), Err(TrySendError::Full(_))));
    }

    #[tokio::test]
    async fn test_try_recv_empty() {
        let (_tx, rx) = channel::<i32>(1);

        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[tokio::test]
    async fn test_try_recv_closed() {
        let (tx, rx) = channel::<i32>(1);

        drop(tx);

        assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
    }

    #[tokio::test]
    async fn test_multiple_senders() {
        let (tx1, rx) = channel(3);
        let tx2 = tx1.clone();

        tx1.send(1).await.unwrap();
        tx2.send(2).await.unwrap();

        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(rx.recv().await, Some(2));

        drop(tx1);
        assert_eq!(rx.recv().await.is_some(), false); // Should not close yet

        drop(tx2);
        assert_eq!(rx.recv().await, None); // Now closed
    }

    #[tokio::test]
    async fn test_unbuffered_channel() {
        let (tx, rx) = channel(0);

        // Spawn a task to receive
        let recv_handle = tokio::spawn(async move { rx.recv().await });

        // Give receiver time to block
        tokio::task::yield_now().await;

        // Send should complete immediately since receiver is waiting
        tx.send(42).await.unwrap();

        assert_eq!(recv_handle.await.unwrap(), Some(42));
    }

    #[test]
    fn test_sender_close() {
        let (tx, rx) = channel::<i32>(1);

        tx.close();

        assert!(matches!(tx.try_send(1), Err(TrySendError::Closed(_))));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
    }
}
