// Minimal console-based logger; mirrors the previous winston API
// (error/warn/info/verbose/debug/silly + getForWallet) without external deps.

const LOG_LEVEL = (typeof process !== 'undefined' && process.env && process.env.LOG_LEVEL) || 'info';

const LEVELS = {
  silent: -1, error: 0, warn: 1, info: 2, verbose: 3, debug: 4, silly: 5,
};

const cache = {};

function build(level = LOG_LEVEL, prefix = '') {
  const threshold = LEVELS[level] != null ? LEVELS[level] : LEVELS.info;
  const noop = () => {};
  const fmt = prefix ? (...a) => [prefix, ...a] : (...a) => a;

  const logger = {
    error: threshold >= 0 ? (...a) => console.error(...fmt(...a)) : noop,
    warn: threshold >= 1 ? (...a) => console.warn(...fmt(...a)) : noop,
    info: threshold >= 2 ? (...a) => console.info(...fmt(...a)) : noop,
    verbose: threshold >= 3 ? (...a) => console.debug(...fmt(...a)) : noop,
    debug: threshold >= 4 ? (...a) => console.debug(...fmt(...a)) : noop,
    silly: threshold >= 5 ? (...a) => console.debug(...fmt(...a)) : noop,
    getForWallet(walletId) {
      if (!cache[walletId]) {
        cache[walletId] = build(level, `[Wallet: ${walletId}]`);
      }
      return cache[walletId];
    },
  };
  return logger;
}

const logger = build();

logger.verbose(`Logger uses "${LOG_LEVEL}" level`);

export default logger;
