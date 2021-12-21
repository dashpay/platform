"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const core_1 = require("@oclif/core");
class World extends core_1.Command {
    async run() {
        this.log('hello world! (./src/commands/hello/world.ts)');
    }
}
exports.default = World;
World.description = 'Say hello world';
World.examples = [
    `$ oex hello world
hello world! (./src/commands/hello/world.ts)
`,
];
World.flags = {};
World.args = [];
