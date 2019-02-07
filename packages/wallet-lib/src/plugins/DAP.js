const StandardPlugin = require('./StandardPlugin');

class DAP extends StandardPlugin {
  constructor(opts) {
    super(Object.assign({ type: 'DAP' }, opts));
    this.isValid = false;
  }

  verifyDAP() {
    return this.isValid;
    // TODO: Schema validation ?
    // TODO : Verify if DAP is reachable via DAPI ?
  }
}
module.exports = DAP;
