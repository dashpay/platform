# DAPI Integration tests

These tests are used to test integration of DAPI and dapi-client with other components
and on _real_ network that build with docker images.

## Usage

In presetup tests download required containers( dashcore, dapi, insight, drive, mongodb, ipfs).
You need to provide access to them and set appropriate environment variables before execution
```
AWS_ACCESS_KEY_ID=<>
AWS_SECRET_ACCESS_KEY=<>
AWS_DEFAULT_REGION=<>
AWS_REGION=<>
```

To run all tests under integration
```javascript
npm run test:integration
```
