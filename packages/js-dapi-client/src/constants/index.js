module.exports = {
  userEvents: {
    STATE_UPDATED: 'STATE_UPDATED',
  },
  servicesEvents: {
    NEW_BLOCK: 'NEW_BLOCK',
  },
  subTxTypes: {
    REGISTER: 1,
    TOP_UP: 2,
  },
  stateTransitionsTypes: {
    UPDATE_DATA: 1,
    RESET_KEY: 2,
    CLOSE_ACCOUNT: 3,
  },
  evoVersion: 0x00010000,
  storage: {
    NODE: { DEFAULT_STORAGE_PATH: './.storage' },
    BROWSER: { DEFAULT_DB_NAME: 'dashSdkStorage' },
  },
};
