"""Message relay with tidepool buffering."""

import time
from dataclasses import dataclass, field
from collections import deque
from typing import Optional

@dataclass
class Message:
    id: str
    sender: str
    recipient: str
    content: str
    priority: str = "P2"
    timestamp: float = field(default_factory=time.time)
    delivered: bool = False

class TidePool:
    def __init__(self, max_size: int = 1000, flush_interval: float = 300.0):
        self.max_size = max_size
        self.flush_interval = flush_interval
        self._buffer: deque = deque(maxlen=max_size)
        self._last_flush = time.time()

    def add(self, message: Message):
        self._buffer.append(message)

    def flush(self) -> list[Message]:
        messages = list(self._buffer)
        self._buffer.clear()
        self._last_flush = time.time()
        return messages

    def should_flush(self) -> bool:
        return len(self._buffer) >= self.max_size * 0.8 or (time.time() - self._last_flush) >= self.flush_interval

    def peek(self, n: int = 5) -> list[Message]:
        return list(self._buffer)[-n:]

    @property
    def stats(self) -> dict:
        return {"buffered": len(self._buffer), "capacity": self.max_size,
                "should_flush": self.should_flush()}

class Relay:
    def __init__(self, agent_id: str):
        self.agent_id = agent_id
        self._outbox: list[Message] = []
        self._inbox: list[Message] = []
        self._tidepool = TidePool()
        self._delivered: list[Message] = []

    def send(self, recipient: str, content: str, priority: str = "P2") -> Message:
        msg = Message(id=f"{self.agent_id}-{len(self._outbox)}", sender=self.agent_id,
                      recipient=recipient, content=content, priority=priority)
        self._outbox.append(msg)
        self._tidepool.add(msg)
        return msg

    def receive(self, message: Message):
        self._inbox.append(message)

    def drain_inbox(self) -> list[Message]:
        messages = self._inbox
        self._inbox = []
        for m in messages:
            m.delivered = True
            self._delivered.append(m)
        return messages

    def flush_tidepool(self) -> list[Message]:
        if self._tidepool.should_flush():
            return self._tidepool.flush()
        return []

    def get_conversation(self, agent_id: str, limit: int = 50) -> list[Message]:
        all_msgs = [(m, m.timestamp) for m in self._outbox + self._delivered if m.recipient == agent_id]
        all_msgs += [(m, m.timestamp) for m in self._delivered if m.sender == agent_id]
        all_msgs.sort(key=lambda x: x[1])
        return [m for m, _ in all_msgs[-limit:]]

    @property
    def stats(self) -> dict:
        return {"outbox": len(self._outbox), "inbox": len(self._inbox),
                "delivered": len(self._delivered), "tidepool": self._tidepool.stats}
