const core = require('@actions/core');
const github = require('@actions/github');

const image = core.getInput('image');
const tag = core.getInput('tag');

const manifestNames = [
  `${image}_sha_${github.context.sha}`,
  `${image}_tag_${tag}`,
  `${image}`
];

const settings = {
  type: 's3',
  region: core.getInput('region'),
  bucket: core.getInput('bucket'),
  prefix: core.getInput('prefix'),
  name: manifestNames.join(';'),
};

const settingsString = Object.entries(settings)
  .filter(([,value]) => value !== '')
  .map(([key, value]) => `${key}=${value}`)
  .join(',');

core.setOutput('cache_from', settingsString);
core.setOutput('cache_to', `${settingsString},mode=${core.getInput('mode')}`);
