# One Billion Rows Challenge - Rust Edition

The goal is to process 1 billion rows worth of data as fast as possible.

A few techniques where used in this program : generally speaking, the goal is to prevent repetitive allocations on heap as they are really slow. I/O still is our main bottlenech.