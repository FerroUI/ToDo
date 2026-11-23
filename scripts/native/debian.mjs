import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

const packageTerm = /^[a-z0-9][a-z0-9+.-]*(?::[a-z0-9-]+)?(?: \((?:<<|<=|=|>=|>>) [0-9A-Za-z.+:~_-]+\))?$/;

export function linkedDebianDependencies(output) {
    const lines = output.split(/\r?\n/).filter(line => line.startsWith("shlibs:Depends="));
    if (lines.length !== 1) throw new Error("dpkg-shlibdeps did not return one dependency declaration");
    const dependencies = lines[0].slice("shlibs:Depends=".length).split(",").map(value => value.trim());
    if (!dependencies.length || dependencies.some(clause => !clause.split("|").every(term => packageTerm.test(term.trim())))) {
        throw new Error("dpkg-shlibdeps returned an invalid dependency declaration");
    }
    return [...new Set(dependencies)];
}

export function debianDependencies(minimumGlibc, linked = []) {
    if (!/^\d+\.\d+(?:\.\d+)?$/.test(minimumGlibc)) throw new Error("Invalid minimum glibc version");
    if (!Array.isArray(linked) || linked.some(clause => typeof clause !== "string" || /[\u0000-\u001f\u007f]/.test(clause)
        || !clause.split("|").every(term => packageTerm.test(term.trim())))) {
        throw new Error("Invalid linked-library dependency declaration");
    }
    // The C++/GCC requirements come from each executable's real imported symbols.
    // Keep explicit GUI dependencies too: dynamically loaded libraries do not all
    // appear in ELF DT_NEEDED. Preserve separate version constraints as conjunctions.
    const direct = new Set(linked.filter(clause => !clause.includes("|")).map(clause => clause.split(" ")[0]));
    const gui = ["libstdc++6", "libgcc-s1", "libfontconfig1", "libfreetype6", "libx11-6", "libxi6",
        "libxrandr2", "libxcursor1", "libxrender1", "libxext6", "libegl1", "libgl1", "libgtk-3-0"];
    return [...new Set([`libc6 (>= ${minimumGlibc})`, ...linked, ...gui.filter(name => !direct.has(name))])].join(", ");
}

export async function inspectDebianDependencies(executables, work, env, runner) {
    const directory = join(work, "shlibdeps");
    await mkdir(join(directory, "debian"), { recursive: true });
    // dpkg-shlibdeps needs source/binary control metadata even when writing only to
    // stdout. It uses the build host's package symbols database and fails if a
    // linked non-system library has no dependency information; never ignore that.
    await writeFile(join(directory, "debian/control"), "Source: ferroui-todo\nSection: utils\nPriority: optional\nMaintainer: FerroUI contributors <noreply@ferroui.com>\n\nPackage: ferroui-todo\nArchitecture: any\nDescription: FerroUI task manager\n Native runtime dependency inspection.\n");
    const output = await runner("dpkg-shlibdeps", ["-O", ...executables.map(path => `-e${path}`)],
        { cwd: directory, env, capture: true });
    return linkedDebianDependencies(output);
}
