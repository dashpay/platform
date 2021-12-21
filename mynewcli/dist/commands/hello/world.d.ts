import { Command } from '@oclif/core';
export default class World extends Command {
    static description: string;
    static examples: string[];
    static flags: {};
    static args: never[];
    run(): Promise<void>;
}
