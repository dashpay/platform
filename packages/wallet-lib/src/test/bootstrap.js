const path = require('path');
const dotenvSafe = require('dotenv-safe');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
