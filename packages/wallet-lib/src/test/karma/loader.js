// This file is used for compiling tests with webpack into one file for using with karma
require('./bootstrap');

const testsContext = require.context('../../../src', true, /spec.js$/);
const integrationTestsContext = require.context('../../../tests/integration', true, /spec.js$/);

testsContext.keys().forEach(testsContext);
integrationTestsContext.keys().forEach(integrationTestsContext);
