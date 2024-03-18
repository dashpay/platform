// This file is used for compiling tests with webpack into one file for using with karma
require('./bootstrap');

// noinspection JSUnresolvedFunction
const testsContext = require.context('../../../test', true, /^(?!.*functional).*\.js$/);

testsContext.keys().forEach(testsContext);
