const config = require('../config');
const Insight = require('./insight');

const insight = new Insight({ uri: config.insightUri });

module.exports = insight;
