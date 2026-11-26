#!/usr/bin/env node
// Ledger of the qa-fix-loop skill: one entry per defect, from "found" to
// "passed on a device". It lives outside the repository, in the QA home
// directory, and has one writer: the coordinator.
//
//   node ledger.mjs --home <dir> <command> [args]     (or set QA_HOME)
//
// Run `node ledger.mjs help` for the commands. `sync` and `next` are the two
// the loop runs on every tick: `sync` reads pull request states from GitHub
// with `gh`, `next` prints what is due.

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const GH = process.env.GH_BIN || 'gh';
const STATES = ['found', 'threaded', 'pr-open', 'merged', 'failed', 'passed', 'closed', 'decision'];
const APPS = ['swift', 'kotlin'];
const CLOSE_REASONS = ['duplicate', 'not-a-bug', 'by-design', 'fixture', 'infra', 'wont-fix'];
const NUDGE_AFTER_MINUTES = 60;
const MAX_NUDGES = 2;

const HELP = `Usage: node ledger.mjs [--home <dir>] <command>

  init --repo <owner/name> --base <branch> --network <name> [--deployed-ref <tag|sha>]
  add --id <slug> --title <text> [--apps swift,kotlin] [--rows A-01,B-02]
      [--source <where it was found>] [--state found|decision|closed]
      [--reason <${CLOSE_REASONS.join('|')}>] [--no-retest] [--note <text>]
  set <id> key=value ...      keys: thread model branch pr state reason note title
                              rows apps retest nudges   (pr= clears the PR)
  nudged <id>                 count a nudge sent to the item's fix thread
  sync                        read each item's pull request and the base head from GitHub
  next                        print the actions that are due, one per line
  retest-start --app <app> --commit <sha> --ids a,b [--agent <ref>]
  retest-result <id> --app <app> --result pass|fail [--evidence <text>]
  table                       the ledger as a Markdown table
  show <id>                   one item as JSON

States: ${STATES.join(', ')}.
An item is finished when it is passed, closed, or merged with nothing left to retest.`;

function fail(message) {
  console.error(`ledger: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg.startsWith('--')) {
      const name = arg.slice(2);
      const next = argv[i + 1];
      if (next === undefined || next.startsWith('--')) {
        flags[name] = true;
      } else {
        flags[name] = next;
        i += 1;
      }
    } else {
      positional.push(arg);
    }
  }
  return { flags, positional };
}

function list(value) {
  if (value === undefined || value === true || value === '') return [];
  return String(value).split(',').map((s) => s.trim()).filter(Boolean);
}

function now() {
  return new Date().toISOString();
}

function minutesSince(iso) {
  return iso ? Math.floor((Date.now() - Date.parse(iso)) / 60000) : 0;
}

function ledgerPath(home) {
  return path.join(home, 'ledger.json');
}

function load(home) {
  const file = ledgerPath(home);
  if (!fs.existsSync(file)) fail(`no ledger at ${file}; run init first`);
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function save(home, ledger) {
  const file = ledgerPath(home);
  const tmp = `${file}.tmp-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(ledger, null, 2)}\n`);
  fs.renameSync(tmp, file);
}

function find(ledger, id) {
  const item = ledger.items.find((i) => i.id === id);
  if (!item) fail(`no item "${id}"`);
  return item;
}

function setState(item, state) {
  if (!STATES.includes(state)) fail(`unknown state "${state}"; one of ${STATES.join(', ')}`);
  if (item.state !== state) {
    item.state = state;
    item.stateAt = now();
  }
}

function checkApps(apps) {
  for (const app of apps) {
    if (!APPS.includes(app)) fail(`unknown app "${app}"; one of ${APPS.join(', ')}`);
  }
}

function lastResult(item, app) {
  return item.retests?.[app]?.result;
}

function retestRunning(item, app) {
  const run = item.retests?.[app];
  return Boolean(run && run.startedAt && !run.result);
}

function appsToRetest(item) {
  if (item.retest === false) return [];
  return item.apps.filter((app) => lastResult(item, app) !== 'pass');
}

function finished(item) {
  if (item.state === 'passed' || item.state === 'closed') return true;
  return item.state === 'merged' && appsToRetest(item).length === 0;
}

function gh(args) {
  return execFileSync(GH, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
}

const PR_FIELDS = 'number,state,isDraft,mergeCommit,mergedAt,url,title,headRefOid,baseRefName';

function readPullRequest(repo, item) {
  if (item.pr) {
    return JSON.parse(gh(['pr', 'view', String(item.pr), '--repo', repo, '--json', PR_FIELDS]));
  }
  if (item.branch) {
    const found = JSON.parse(gh([
      'pr', 'list', '--repo', repo, '--head', item.branch, '--state', 'all',
      '--limit', '10', '--json', PR_FIELDS,
    ]));
    found.sort((a, b) => b.number - a.number);
    return found[0];
  }
  return undefined;
}

function applyPullRequest(item, pr) {
  item.pr = pr.number;
  item.prUrl = pr.url;
  item.prTitle = pr.title;
  item.prState = pr.state;
  item.prDraft = pr.isDraft;
  item.prHead = pr.headRefOid;
  item.prBase = pr.baseRefName;
  if (pr.state === 'MERGED') {
    item.mergeCommit = pr.mergeCommit?.oid;
    item.mergedAt = pr.mergedAt;
    setState(item, 'merged');
  } else {
    setState(item, 'pr-open');
  }
}

const commands = {
  help() {
    console.log(HELP);
  },

  init(home, { flags }) {
    for (const required of ['repo', 'base', 'network']) {
      if (!flags[required] || flags[required] === true) fail(`init needs --${required}`);
    }
    if (fs.existsSync(ledgerPath(home))) fail(`${ledgerPath(home)} exists; it is never overwritten`);
    fs.mkdirSync(home, { recursive: true });
    save(home, {
      version: 1,
      repo: flags.repo,
      base: flags.base,
      network: flags.network,
      deployedRef: flags['deployed-ref'] === true ? undefined : flags['deployed-ref'],
      createdAt: now(),
      items: [],
    });
    console.log(`created ${ledgerPath(home)}`);
  },

  add(home, { flags }) {
    const ledger = load(home);
    if (!flags.id || flags.id === true) fail('add needs --id');
    if (!flags.title || flags.title === true) fail('add needs --title');
    if (ledger.items.some((i) => i.id === flags.id)) fail(`item "${flags.id}" exists`);
    const apps = list(flags.apps);
    checkApps(apps);
    const state = flags.state && flags.state !== true ? flags.state : 'found';
    if (!['found', 'decision', 'closed'].includes(state)) fail('add takes --state found, decision or closed');
    if (state === 'closed' && !CLOSE_REASONS.includes(flags.reason)) {
      fail(`a closed item needs --reason, one of ${CLOSE_REASONS.join(', ')}`);
    }
    const item = {
      id: flags.id,
      title: flags.title,
      apps,
      rows: list(flags.rows),
      source: flags.source === true ? undefined : flags.source,
      state,
      stateAt: now(),
      createdAt: now(),
      reason: flags.reason === true ? undefined : flags.reason,
      note: flags.note === true ? undefined : flags.note,
      retest: flags['no-retest'] ? false : apps.length > 0,
      nudges: 0,
      retests: {},
    };
    ledger.items.push(item);
    save(home, ledger);
    console.log(`added ${item.id} (${item.state})`);
  },

  set(home, { positional }) {
    const ledger = load(home);
    const [id, ...pairs] = positional;
    if (!id || pairs.length === 0) fail('set needs an id and key=value pairs');
    const item = find(ledger, id);
    for (const pair of pairs) {
      const at = pair.indexOf('=');
      if (at < 1) fail(`"${pair}" is not key=value`);
      const key = pair.slice(0, at);
      const value = pair.slice(at + 1);
      switch (key) {
        case 'state':
          setState(item, value);
          break;
        case 'pr':
          if (value === '') {
            for (const k of ['pr', 'prUrl', 'prTitle', 'prState', 'prDraft', 'prHead', 'prBase', 'mergeCommit', 'mergedAt']) {
              delete item[k];
            }
          } else if (/^\d+$/.test(value)) {
            item.pr = Number(value);
          } else {
            fail('pr takes a number, or nothing to clear it');
          }
          break;
        case 'rows':
          item.rows = list(value);
          break;
        case 'apps':
          checkApps(list(value));
          item.apps = list(value);
          break;
        case 'retest':
          if (!['true', 'false'].includes(value)) fail('retest takes true or false');
          item.retest = value === 'true';
          break;
        case 'nudges':
          item.nudges = Number(value);
          break;
        case 'reason':
          if (!CLOSE_REASONS.includes(value)) fail(`reason is one of ${CLOSE_REASONS.join(', ')}`);
          item.reason = value;
          break;
        case 'thread':
        case 'model':
        case 'branch':
        case 'note':
        case 'title':
          item[key] = value;
          break;
        default:
          fail(`unknown key "${key}"`);
      }
    }
    if (item.state === 'closed' && !item.reason) fail('a closed item needs a reason');
    save(home, ledger);
    console.log(`updated ${item.id} (${item.state})`);
  },

  nudged(home, { positional }) {
    const ledger = load(home);
    const item = find(ledger, positional[0] ?? '');
    item.nudges = (item.nudges ?? 0) + 1;
    item.nudgedAt = now();
    save(home, ledger);
    console.log(`${item.id}: ${item.nudges} nudge(s)`);
  },

  sync(home) {
    const ledger = load(home);
    const changes = [];
    try {
      ledger.baseHead = gh(['api', `repos/${ledger.repo}/branches/${ledger.base}`, '--jq', '.commit.sha']).trim();
      delete ledger.syncError;
    } catch (error) {
      ledger.syncError = String(error.stderr || error.message).trim();
    }
    for (const item of ledger.items) {
      if (!['threaded', 'pr-open'].includes(item.state)) continue;
      const before = `${item.state}/${item.pr ?? '-'}/${item.prState ?? '-'}/${item.prDraft ?? '-'}`;
      try {
        const pr = readPullRequest(ledger.repo, item);
        if (pr) applyPullRequest(item, pr);
        delete item.syncError;
      } catch (error) {
        item.syncError = String(error.stderr || error.message).trim();
      }
      const after = `${item.state}/${item.pr ?? '-'}/${item.prState ?? '-'}/${item.prDraft ?? '-'}`;
      if (before !== after) changes.push(`${item.id}: ${before} -> ${after}`);
      if (item.syncError) changes.push(`${item.id}: could not read its pull request: ${item.syncError}`);
    }
    ledger.syncedAt = now();
    save(home, ledger);
    if (ledger.syncError) console.log(`base head not read: ${ledger.syncError}`);
    console.log(`${ledger.base} head ${ledger.baseHead ?? 'unknown'}`);
    console.log(changes.length ? changes.join('\n') : 'no pull request changed state');
  },

  next(home) {
    const ledger = load(home);
    const lines = [];
    for (const item of ledger.items) {
      const pr = item.pr ? `#${item.pr}` : 'no PR';
      if (item.state === 'found') {
        lines.push(`THREAD ${item.id}: no fix thread yet. "${item.title}"`);
      } else if (item.state === 'threaded') {
        const waited = minutesSince(item.nudgedAt ?? item.stateAt);
        if (waited >= NUDGE_AFTER_MINUTES && (item.nudges ?? 0) < MAX_NUDGES) {
          lines.push(`NUDGE ${item.id}: thread ${item.thread ?? '?'} has no pull request after ${waited} min. Read it first; nudge only if it stopped.`);
        } else if ((item.nudges ?? 0) >= MAX_NUDGES && waited >= NUDGE_AFTER_MINUTES) {
          lines.push(`HUMAN ${item.id}: thread ${item.thread ?? '?'} has no pull request after ${MAX_NUDGES} nudges.`);
        }
      } else if (item.state === 'pr-open' && item.prState === 'CLOSED') {
        lines.push(`ASK ${item.id}: ${pr} was closed without a merge. Ask thread ${item.thread ?? '?'} why, then close or re-thread the item.`);
      } else if (item.state === 'failed') {
        lines.push(`TRIAGE ${item.id}: its retest failed. Send the evidence to thread ${item.thread ?? '?'} for a follow-up pull request, then set state=threaded branch=<new> pr=`);
      } else if (item.state === 'decision') {
        lines.push(`HUMAN ${item.id}: waits on a decision. "${item.title}"`);
      }
    }
    for (const app of APPS) {
      const mine = ledger.items.filter((i) => i.apps.includes(app));
      const running = mine.filter((i) => retestRunning(i, app));
      const pending = mine.filter((i) => i.state === 'merged' && appsToRetest(i).includes(app) && !retestRunning(i, app));
      const inFlight = mine.filter((i) => ['found', 'threaded', 'pr-open', 'failed'].includes(i.state));
      if (running.length) {
        const since = minutesSince(running[0].retests[app].startedAt);
        lines.push(`RUNNING ${app}: retest of ${running.map((i) => i.id).join(', ')} for ${since} min.`);
      } else if (pending.length && inFlight.length === 0) {
        const rows = [...new Set(pending.flatMap((i) => i.rows))].join(', ') || 'see the items';
        lines.push(`RETEST ${app}: every fix is merged. Items ${pending.map((i) => i.id).join(', ')}; rows ${rows}; build ${ledger.base} at ${ledger.baseHead ?? '(run sync)'}.`);
      } else if (pending.length) {
        const waiting = inFlight.map((i) => (i.pr ? `${i.id} #${i.pr}` : `${i.id} (${i.state})`)).join(', ');
        lines.push(`HOLD ${app}: ${pending.length} merged (${pending.map((i) => i.id).join(', ')}), waiting on ${waiting}. Retest now only if those cannot change this app's build.`);
      }
    }
    const open = ledger.items.filter((i) => !finished(i) && i.state !== 'decision');
    if (ledger.items.length && open.length === 0) {
      lines.push('DONE: every item is passed, closed or merged with nothing to retest. Write the final report and remove the scheduled check.');
    }
    if (ledger.syncedAt === undefined || minutesSince(ledger.syncedAt) > 10) {
      lines.unshift(`STALE: last sync ${ledger.syncedAt ?? 'never'}. Run sync first.`);
    }
    console.log(lines.length ? lines.join('\n') : 'nothing is due');
  },

  'retest-start': (home, { flags }) => {
    const ledger = load(home);
    const app = flags.app;
    checkApps([app]);
    if (!flags.commit || flags.commit === true) fail('retest-start needs --commit');
    const ids = list(flags.ids);
    if (ids.length === 0) fail('retest-start needs --ids');
    for (const id of ids) {
      const item = find(ledger, id);
      if (!item.apps.includes(app)) fail(`${id} is not an item of the ${app} app`);
      item.retests = item.retests ?? {};
      item.retests[app] = {
        commit: flags.commit,
        agent: flags.agent === true ? undefined : flags.agent,
        startedAt: now(),
      };
    }
    save(home, ledger);
    console.log(`retest of ${ids.join(', ')} on ${app} at ${flags.commit} recorded as running`);
  },

  'retest-result': (home, { flags, positional }) => {
    const ledger = load(home);
    const item = find(ledger, positional[0] ?? '');
    const app = flags.app;
    checkApps([app]);
    if (!['pass', 'fail'].includes(flags.result)) fail('retest-result needs --result pass or fail');
    const run = item.retests?.[app];
    if (!run) fail(`${item.id} has no retest started on ${app}`);
    run.result = flags.result;
    run.finishedAt = now();
    if (flags.evidence && flags.evidence !== true) run.evidence = flags.evidence;
    if (flags.result === 'fail') {
      setState(item, 'failed');
    } else if (item.state === 'merged' && appsToRetest(item).length === 0) {
      setState(item, 'passed');
    }
    save(home, ledger);
    console.log(`${item.id}: ${app} ${flags.result} at ${run.commit} (${item.state})`);
  },

  table(home) {
    const ledger = load(home);
    const cell = (text) => String(text ?? '').replace(/\|/g, '\\|');
    console.log(`Ledger of ${ledger.network} (${ledger.repo}, base ${ledger.base}${ledger.baseHead ? ` at ${ledger.baseHead.slice(0, 10)}` : ''}), synced ${ledger.syncedAt ?? 'never'}\n`);
    console.log('| Item | Apps | Rows | State | PR | Retest | Thread | Title |');
    console.log('|---|---|---|---|---|---|---|---|');
    for (const item of ledger.items) {
      const pr = item.pr ? `#${item.pr}${item.prDraft && item.prState === 'OPEN' ? ' (draft)' : ''}` : '';
      const retest = item.retest === false
        ? 'none'
        : item.apps.map((app) => {
          const run = item.retests?.[app];
          if (!run) return `${app}: -`;
          return `${app}: ${run.result ?? 'running'} @${String(run.commit).slice(0, 10)}`;
        }).join('; ');
      const state = item.state === 'closed' ? `closed (${item.reason})` : item.state;
      console.log(`| ${cell(item.id)} | ${cell(item.apps.join(', '))} | ${cell(item.rows.join(', '))} | ${cell(state)} | ${pr} | ${cell(retest)} | ${cell(item.thread)} | ${cell(item.title)} |`);
    }
  },

  show(home, { positional }) {
    console.log(JSON.stringify(find(load(home), positional[0] ?? ''), null, 2));
  },
};

const { flags, positional } = parseArgs(process.argv.slice(2));
const [command = 'help', ...rest] = positional;
if (command === 'help' || flags.help) {
  commands.help();
} else {
  const home = flags.home && flags.home !== true ? flags.home : process.env.QA_HOME;
  if (!home) fail('give the QA home directory with --home or QA_HOME');
  if (!commands[command]) fail(`unknown command "${command}"; run help`);
  commands[command](path.resolve(home), { flags, positional: rest });
}
