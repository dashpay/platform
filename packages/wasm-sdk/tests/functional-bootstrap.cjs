// Allow self-signed certificates for local dashmate nodes (functional tests only)
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

// Load shared test bootstrap
require('./bootstrap.cjs');
