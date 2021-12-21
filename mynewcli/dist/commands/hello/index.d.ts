import { Command } from '@oclif/core';
export default class Hello extends Command {
    static description: string;
    static examples: string[];
    static flags: {
        from: import("@oclif/core/lib/interfaces").OptionFlag<string>;
    };
    static args: {
        name: string;
        description: string;
        required: boolean;
    }[];
    run(): Promise<void>;
}
