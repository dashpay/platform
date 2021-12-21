"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const core_1 = require("@oclif/core");
class Hello extends core_1.Command {
    async run() {
        const { args, flags } = await this.parse(Hello);
        this.log(`hello ${args.person} from ${flags.from}! (./src/commands/hello/index.ts)`);
    }
}
exports.default = Hello;
Hello.description = 'Say hello';
Hello.examples = [
    `$ oex hello friend --from oclif
hello friend from oclif! (./src/commands/hello/index.ts)
`,
];
Hello.flags = {
    from: core_1.Flags.string({ char: 'f', description: 'Whom is saying hello', required: true }),
};
Hello.args = [{ name: 'person', description: 'Person to say hello to', required: true }];
