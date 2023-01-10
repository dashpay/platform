const { execSync } = require('child_process');

async function execute(cmd, opts) {
  try {
    const result = execSync(cmd, opts)
    return result;
  } catch (e) {
    return e;
  }
}

module.exports = { execute };
