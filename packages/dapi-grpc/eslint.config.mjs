import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  ...baseConfig,
  mochaTestConfig,
  {
    // Generated protobuf code - ignore clients and src generated directories
    ignores: ['dist/**', 'node_modules/**', 'clients/**', 'src/**'],
  },
];
