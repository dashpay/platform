#!/usr/bin/env python3
"""Bound Binaryen's thread pool without changing package build commands/passes."""
import os
from pathlib import Path
import re
import sys


def cpu_budget(cgroup=Path('/sys/fs/cgroup'), affinity=None):
    """Container CPU-time quota and affinity, not the host's hardware count."""
    if affinity is None:
        affinity = len(os.sched_getaffinity(0))
    limits = [affinity]
    maximum = cgroup / 'cpu.max'
    if maximum.exists():
        quota, period = maximum.read_text().split()
        if quota != 'max':
            limits.append(max(1, int(quota) // int(period)))
    else:
        # Docker cgroup v1 may mount the cpu controller under either name.
        for controller in (cgroup / 'cpu', cgroup / 'cpu,cpuacct'):
            quota_file = controller / 'cpu.cfs_quota_us'
            if quota_file.exists():
                quota = int(quota_file.read_text())
                period = int((controller / 'cpu.cfs_period_us').read_text())
                if quota > 0:
                    limits.append(max(1, quota // period))
                break
    return min(limits)


def build_environment(environment, budget):
    env = dict(environment)
    # Compile jobs can be memory-limited independently of the optimizer pool.
    # Preserve an explicit measured Binaryen setting supplied by the runner.
    requested = env.get('BINARYEN_CORES', str(budget))
    if not re.fullmatch(r'[1-9][0-9]*', requested):
        raise ValueError('BINARYEN_CORES must be a positive integer')
    if int(requested) > budget:
        raise ValueError('BINARYEN_CORES exceeds the container CPU budget')
    env['BINARYEN_CORES'] = requested
    return env


def main():
    try:
        budget = cpu_budget()
        env = build_environment(os.environ, budget)
    except (OSError, ValueError, ZeroDivisionError) as error:
        print(f'::error::Cannot select WASM build thread budget: {error}', file=sys.stderr)
        return 2
    print(f'::notice::NPM build: CPU budget={budget}, '
          f'Binaryen threads={env["BINARYEN_CORES"]}; Cargo configuration unchanged',
          flush=True)
    # Bash is already required by this CI action. Its time keyword needs no
    # additional package and preserves yarn's exit status. Cancellation remains
    # the runner's existing process-tree cleanup responsibility.
    # The command and all optimization flags remain unchanged.
    env['TIMEFORMAT'] = 'NPM build timing: wall=%3R user=%3U system=%3S seconds'
    os.execve('/bin/bash', ['bash', '-c', 'time yarn build'], env)


if __name__ == '__main__':
    sys.exit(main())
