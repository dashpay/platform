import formatDkgStopStatus, {
  formatDkgStopRefusal,
} from '../../../../src/core/quorum/formatDkgStopStatus.js';

// Eight-digit heights, the longest these lines are made to fit.
const HEIGHT = 12345678;
const MAX_LINE_LENGTH = 64;

const MAINNET_BLOCK_SECONDS = 150;

function session(from, extra = {}) {
  return {
    reason: 'session', llmqType: 'llmq_100_67', quorumIndex: 0, from, until: from + 10, ...extra,
  };
}

function upcoming(start, extra = {}) {
  return {
    reason: 'upcoming',
    llmqType: 'llmq_100_67',
    quorumIndex: 0,
    membershipKnown: true,
    from: start - 6,
    until: start + 10,
    ...extra,
  };
}

function expectLinesFit(text) {
  text.split('\n').forEach((line) => {
    expect(line.length, `"${line}" is too long`).to.be.at.most(MAX_LINE_LENGTH);
  });
}

function blocked(blockers, blockTimeSeconds = MAINNET_BLOCK_SECONDS) {
  const text = formatDkgStopStatus({ type: 'blocked', height: HEIGHT, blockers }, { blockTimeSeconds });
  expectLinesFit(text);

  return text.split('\n');
}

describe('formatDkgStopStatus', () => {
  it('should say what blocks and when stopping becomes safe', () => {
    expect(blocked([session(HEIGHT - 3)])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 3} runs until block ${HEIGHT + 7}`,
      `Earliest safe stop: block ${HEIGHT + 7}, in 7+ blocks (~18m)`,
    ]);
  });

  it('should describe every kind of blocker on a line that fits the renderer', () => {
    expect(blocked([
      session(HEIGHT - 3, { llmqType: 'llmq_60_75', quorumIndex: 31 }),
      {
        reason: 'new-round', llmqType: 'llmq_400_85', quorumIndex: 0, from: HEIGHT, until: HEIGHT + 20,
      },
      upcoming(HEIGHT + 4, { llmqType: 'llmq_60_75', quorumIndex: 31, membershipKnown: false }),
      upcoming(HEIGHT + 6, { llmqType: 250, until: null }),
      { reason: 'imminent', from: HEIGHT - 1, until: null },
      {
        reason: 'active-count', count: 3, from: null, until: null,
      },
      {
        reason: 'unreadable', source: 'quorum dkgstatus session llmq_future_unknown', from: null, until: null,
      },
    ])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_60_75 #31 DKG at block ${HEIGHT - 3} runs until block ${HEIGHT + 7}`,
      `llmq_400_85 DKG at block ${HEIGHT} runs until block ${HEIGHT + 20}`,
      `llmq_60_75 #31 DKG starts at block ${HEIGHT + 4} (membership unknown)`,
      `LLMQ type 250 DKG starts at block ${HEIGHT + 6} (member)`,
      `A DKG starts at block ${HEIGHT + 5} (membership not reported)`,
      'Core reports 3 DKG sessions in progress for this node',
      'dashmate cannot read Core\'s DKG data:',
      'quorum dkgstatus session llmq_future_unknown',
      'Earliest safe stop: unknown',
    ]);
  });

  it('should list the later DKGs the estimate waits for', () => {
    expect(blocked([
      session(HEIGHT - 3),
      upcoming(HEIGHT + 12, { llmqType: 'llmq_60_75', quorumIndex: 5 }),
      upcoming(HEIGHT + 100),
    ])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 3} runs until block ${HEIGHT + 7}`,
      `llmq_60_75 #5 DKG starts at block ${HEIGHT + 12} (member)`,
      `Earliest safe stop: block ${HEIGHT + 22}, in 22+ blocks (~55m)`,
    ]);
  });

  it('should describe a DKG it saw announced as running once it has started', () => {
    expect(blocked([upcoming(HEIGHT - 2)])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 2} runs until block ${HEIGHT + 8}`,
      `Earliest safe stop: block ${HEIGHT + 8}, in 8+ blocks (~20m)`,
    ]);
  });

  it('should list a DKG once when Core shows a record of the session it saw announced', () => {
    expect(blocked([
      { ...session(HEIGHT - 2), reason: 'new-round' },
      upcoming(HEIGHT - 2),
    ])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 2} runs until block ${HEIGHT + 8}`,
      `Earliest safe stop: block ${HEIGHT + 8}, in 8+ blocks (~20m)`,
    ]);
  });

  it('should describe a session Core has not recorded yet as running', () => {
    expect(blocked([{
      reason: 'unrecorded', llmqType: 'llmq_100_67', quorumIndex: 0, from: HEIGHT - 2, until: HEIGHT + 8,
    }])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 2} runs until block ${HEIGHT + 8}`,
      `Earliest safe stop: block ${HEIGHT + 8}, in 8+ blocks (~20m)`,
    ]);
  });

  it('should list one session once however many ways it is known', () => {
    // Announced earlier, still shown as last round's record, and not yet
    // recorded as its own.
    expect(blocked([
      upcoming(HEIGHT - 2),
      { ...session(HEIGHT - 2), reason: 'new-round' },
      { ...session(HEIGHT - 2), reason: 'unrecorded' },
    ])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 2} runs until block ${HEIGHT + 8}`,
      `Earliest safe stop: block ${HEIGHT + 8}, in 8+ blocks (~20m)`,
    ]);
  });

  it('should leave out Core\'s session count when the sessions it lists account for it', () => {
    const count = {
      reason: 'active-count', count: 1, from: null, until: null,
    };

    expect(blocked([session(HEIGHT - 3), count])).to.deep.equal([
      `At block ${HEIGHT}, waiting for:`,
      `llmq_100_67 DKG at block ${HEIGHT - 3} runs until block ${HEIGHT + 7}`,
      `Earliest safe stop: block ${HEIGHT + 7}, in 7+ blocks (~18m)`,
    ]);
    expect(blocked([count]).slice(1)).to.deep.equal([
      'Core reports 1 DKG session in progress for this node',
      'Earliest safe stop: unknown',
    ]);
  });

  it('should give long waits in hours and short ones as at least a minute', () => {
    const wait = (blocks, blockTimeSeconds) => blocked([session(HEIGHT - 10 + blocks)], blockTimeSeconds).at(-1);

    expect(wait(10, MAINNET_BLOCK_SECONDS)).to.match(/in 10\+ blocks \(~25m\)$/);
    expect(wait(4, 20)).to.match(/in 4\+ blocks \(~1m\)$/);
    expect(wait(1, 20)).to.match(/in 1\+ blocks \(~1m\)$/);
    expect(wait(10, 600)).to.match(/in 10\+ blocks \(~1h 40m\)$/);
    expect(wait(10, null)).to.match(/in 10\+ blocks$/);
  });

  it('should describe a safe result being confirmed and a state that changed', () => {
    expect(formatDkgStopStatus({ type: 'confirming', height: HEIGHT }, { blockTimeSeconds: null }))
      .to.equal(`Block ${HEIGHT} looks safe; checking again in 15s`);
    expect(formatDkgStopStatus({ type: 'unsettled', height: null }, { blockTimeSeconds: null }))
      .to.equal('Core\'s DKG state changed while reading it; checking again');
  });

  it('should not describe a status it does not know as a failed RPC call', () => {
    expect(formatDkgStopStatus({ type: 'something-new', height: HEIGHT }, { blockTimeSeconds: null }))
      .to.equal('Checking Core\'s DKG state');
  });

  it('should describe a failed RPC call it retries, with the cause on a line of its own', () => {
    const text = formatDkgStopStatus({
      type: 'rpc-error', error: new Error('Work queue depth exceeded'), failedPolls: 2, maxFailedPolls: 6,
    }, { blockTimeSeconds: null });

    expect(text.split('\n')).to.deep.equal([
      'Core RPC failed (2 of 6), retrying:',
      'Work queue depth exceeded',
    ]);
  });
});

describe('formatDkgStopRefusal', () => {
  function refuse(check, flags) {
    const text = formatDkgStopRefusal(check, { blockTimeSeconds: MAINNET_BLOCK_SECONDS, ...flags });
    expectLinesFit(text);

    return text.split('\n');
  }

  const inSession = { status: 'blocked', height: HEIGHT, blockers: [session(HEIGHT - 3)] };

  it('should explain the PoSe risk and offer both flags when the command has them', () => {
    expect(refuse(inSession, { canWait: true, canForce: true })).to.deep.equal([
      'Stopping now risks a PoSe penalty for this masternode:',
      `llmq_100_67 DKG at block ${HEIGHT - 3} runs until block ${HEIGHT + 7}`,
      `Earliest safe stop: block ${HEIGHT + 7}, in 7+ blocks (~18m)`,
      'Retry then, use --safe to wait and stop automatically,',
      'or --force to stop now and accept the PoSe risk',
    ]);
  });

  it('should offer only the flags the command has', () => {
    const hints = (flags) => refuse(inSession, flags).at(-1);

    expect(hints({ canWait: true, canForce: false }))
      .to.equal('Retry then, or use --safe to wait and stop automatically');
    expect(hints({ canWait: false, canForce: true }))
      .to.equal('Retry then, or use --force to stop now and accept the PoSe risk');
    expect(hints({ canWait: false, canForce: false })).to.equal('Retry then');
  });

  it('should say to retry later when it cannot tell when stopping becomes safe', () => {
    const lines = refuse({
      status: 'blocked', height: HEIGHT, blockers: [{ reason: 'imminent', from: HEIGHT - 4, until: null }],
    }, { canWait: false, canForce: true });

    expect(lines.slice(-2)).to.deep.equal([
      'Earliest safe stop: unknown',
      'Retry later, or use --force to stop now and accept the PoSe risk',
    ]);
  });

  it('should not offer --safe for data dashmate cannot read, which does not clear on its own', () => {
    const unreadable = {
      status: 'blocked',
      height: HEIGHT,
      blockers: [{
        reason: 'unreadable', source: 'quorum dkginfo', from: null, until: null,
      }],
    };

    expect(refuse(unreadable, { canWait: true, canForce: true })).to.deep.equal([
      'dashmate cannot read Core\'s DKG data:',
      'quorum dkginfo',
      'Update dashmate, or use --force to stop now',
    ]);
    expect(refuse(unreadable, { canWait: true, canForce: false }).at(-1)).to.equal('Update dashmate');
  });

  it('should explain a state that kept changing', () => {
    expect(refuse({ status: 'unsettled', height: null, blockers: [] }, {
      canWait: true, canForce: true,
    })).to.deep.equal([
      'Core\'s DKG state kept changing while reading it',
      '(is Core still syncing?)',
      'Retry later, use --safe to wait and stop automatically,',
      'or --force to stop now and accept the PoSe risk',
    ]);
  });
});
