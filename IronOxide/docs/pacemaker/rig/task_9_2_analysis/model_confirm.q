// rsk confirmation queries (R31.5): no deadline-miss location is reachable in the stopwatch-free over-approximation,
// and its invariants stop time only after a miss.
A[] not (T0.Missed || T1.Missed || T2.Missed || T3.Missed)
A[] (deadlock imply (T0.Missed || T1.Missed || T2.Missed || T3.Missed))
