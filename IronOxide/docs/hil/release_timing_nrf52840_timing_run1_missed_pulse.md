# The unanswered pulse of the first PAR-05 run (2026-11-01, task 7.3)

The first hour-long release-timing run (`release_timing_nrf52840_timing_run1.md`) passed every
Kernel-side check (823,317 periodic Releases, every nominal instant `O + k*T`, lag at most 6,690
cycles, 13 counter wraps, no storm, the target's `RSK-OK`) and failed one step: of the 203,333
stimulus pulses inside the run, one was not answered. The target counted 203,332 `pulse` Jobs, one
per response edge in the capture, so the Job that answers did not run for that pulse.

## Where

The pulse at 587.879 s into the capture, 581.31 s after the response line was driven low
(Partition 1's first `log` Job, the run's instant 0 within a few milliseconds). The previous pulse
(18 ms earlier) was answered 0.87 ms after its rise, the next (18 ms later) 4.8 ms after its rise.

## What the trace pins show (100 kHz; `task` is the executing Task's index on D1 to D3, `kern` the
Kernel-active flag on D4, `stim` the stimulus mirror on D6, `resp` the response line on D0)

Times are relative to the unanswered pulse's rise.

```
   -0.36 ms  task=0 kern=1   control's dispatch (its 10 ms instant, reached at the end of
                             sensor's 12 ms Critical_Section)
   -0.12 ms  task=1 kern=1   echo
   -0.09 ms  task=2 kern=1   sensor resumes and ends its Job (SVC 3, epilogue)
   -0.01 ms  task=7 kern=0   idle, one sample
    0.00 ms  task=7 kern=1   the stimulus rises; Kernel entry (GPIOTE intake, vector 22, then
                             the pended dispatch vector 40): 70 us
    0.07 ms  task=4 kern=1   pulse's Job dispatched: 40 us with the Kernel flag set (the Job's
                             body is about 1 us; the flag covers its end SVC and the epilogue)
                             -- the response line does NOT toggle, the count does NOT advance
    0.11 ms  task=7 kern=0   idle, one sample
    0.12 ms  task=7 kern=1   Kernel entry: 40 us (sampler's dispatch, vector 56)
    0.16 ms  task=6 kern=1   sampler's Job (70 us), then idle
```

A normal pulse 54 ms earlier, for comparison:

```
   -0.14 ms  task=7 kern=1   intake and dispatch: 70 us
   -0.07 ms  task=4 kern=1   pulse's Job: 30 us, the response line toggles in its first sample
   -0.04 ms  task=7 kern=0   idle
```

The two differ in one thing: the Job's effect. The pulse arrived right after the end of
`sensor`'s Job, at the nominal instant of `control` and `sampler` (both offset 0; `sampler`, at
Priority 1 like `sensor`, waits for `sensor`'s Job of equal Priority to end, and its dispatch vector
is pended again by that Job's epilogue). The hour's trace ring holds only its last 200 ms and the
32-entry event log had been overwritten by the late Jobs' expected events, so whether the Kernel
logged anything for this Job (a kill before its first instruction would log an event; a Job that
ran its body could not fail to toggle the line) is not recoverable from this run.

## What the second run adds

The HIL build now mirrors every Health_Monitor event into no-init RAM (`EVENT_MIRROR`, the last
1,024) and freezes the trace ring at the first event outside the run's expected set (the late
Jobs' own), so a recurrence leaves the event and the 512 Kernel entries leading to it. The
loopback step also reports the instant of every unanswered pulse.
