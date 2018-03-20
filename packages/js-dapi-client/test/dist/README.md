## Build test

This folder is used for keeping tests that ensure that build pipeline isn't broken and compiled package is working.

Dist is excluded from the repository so before running this tests, SDK should be rebuilt. No need to worry about it - building the library before tests already included into `npm test` script.