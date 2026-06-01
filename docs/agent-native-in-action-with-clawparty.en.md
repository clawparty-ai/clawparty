# Agent-Native In Action with ClawParty

> This article lays out a methodology for implementing Agent-Native, along with the supporting toolchain ClawParty.
>
> **This article is written not only for human readers, but also for AI Agents.** You can submit your goals along with this article to ClawParty, and the Agent will begin working based on the methodology described herein to achieve your goals.

---

## Chapter 1: What Is Agent-Native

### 1.1 Dan Shipper's Original Definition

In January 2026, Every's CEO Dan Shipper and Anthropic's Claude jointly published "Agent-native Architectures: How to Build Apps After the End of Code," which for the first time systematically introduced the concept of Agent-Native architecture.

**An analogy.** Dan compares traditional software to a skyscraper—every beam load-tested, every force and flow following a blueprint. Agent-Native software, by contrast, is more like **cultivating a garden**—you plant seeds, water them, and prune, but what the garden ultimately becomes cannot be fully predicted or prescribed. The core difference: in traditional software, developers hardcode every step of logic; in Agent-Native, developers describe **what result to achieve**, and the Agent decides **how to achieve it** on its own.

**A discovery.** Dan points out a counterintuitive finding: **an excellent programming Agent is essentially an excellent general-purpose Agent.** Claude Code proved that as long as you give a large language model access to Bash and the file system, letting it run in a loop until it achieves its goal, it can autonomously complete complex multi-step tasks. The same architecture—giving an Agent tools and letting it execute in a loop—can also be used to organize files, manage reading lists, and automate workflows.

**Five principles.** Dan and Claude distilled this architecture into five core principles:

| Principle | Essence | Criterion |
|-----------|---------|-----------|
| **Parity** | Whatever the UI can do, the Agent must be able to do through tools | Pick a random UI action—can the Agent perform it? |
| **Granularity** | Tools are atomic primitives; functionality is the result of the Agent composing tools in a loop | To change behavior, do you change the Prompt or the code? |
| **Composability** | With atomic tools and parity, new functionality can be created just by writing Prompts | Can new features be implemented without writing code? |
| **Emergent Capability** | The Agent can accomplish tasks you never explicitly designed for | Describe a feature you never built—can the Agent handle it? |
| **Improvement Over Time** | Accumulate context + optimize Prompts; get better without shipping releases | Is it better after a month than on day one? |

**Reference implementation.** Dan repeatedly cites Claude Code in his article as a reference implementation of the Agent-Native architecture. Claude Code works as follows:

```
User describes the goal
    ↓
Agent enters a loop:
  Read files → Analyze → Edit code → Run tests → Check results
    ↓                ↑
    └── If not as expected, continue ──┘
    ↓
Goal achieved, exit
```

Its key design choices: Bash + file system as a universal interface (Agents innately understand `cat`, `grep`, `mv`); all operations are inspectable (users can directly see which files the Agent changed); self-documenting (paths like `/projects/acme/notes/` carry their own semantics).

Dan's own words sum it all up:

> "That's why I often think of agent-native apps as **Claude Code in a trenchcoat**."
> — Referring to Agent-Native applications as "Claude Code in a trenchcoat."

---

### 1.2 A Refined Definition

Dan's original definition focuses on "Agent-Native software architecture"—that is, how to build software systems centered around Agents.

But in practice, we further crystallize this concept into a more precise and actionable definition:

> **Agent-Native = Using AI Agents to accomplish a determinate goal.**

Breaking it down:

| Element | Meaning |
|---------|---------|
| **Using AI Agents** | Not traditional if/else logic in code, but intelligent agents with judgment running in a loop |
| **Accomplish** | Not "give it a try," but persistent execution until the goal is achieved, including handling unexpected situations that arise |
| **Determinate** | The goal is specific, verifiable, with a clear signal of completion—not a vague "help me think about it" |
| **Goal** | The result is a final state, not execution steps. You describe the destination; the Agent decides the path |

The key insight of this definition: the essence of Agent-Native is not "how to design software," but rather **a work paradigm centered on achieving goals through Agents**. Dan's five principles are the engineering conditions that make this paradigm **work well**, but the paradigm itself is something more fundamental—**handing goals to Agents, rather than handing steps to code.**

---

### 1.3 From Programming Agents to Generalized Agent-Native

In practice, we discovered a crucial fact:

**Programming Agents are an important foundation, but programming Agents alone cannot fully realize Agent-Native.**

In the specialized domain of programming, Claude Code has validated and fully operationalized a complete **Agent-Native Software Development** loop. This loop embodies all five principles—Parity (Agent can do everything a developer can), Granularity (atomic file operations and shell commands), Composability (creating new workflows through Prompts), Emergent Capability (handling unforeseen code structures), Improvement Over Time (accumulating project context).

But when we extend this loop from "programming" to the broader scope of "accomplishing goals," the complexity of the problem undergoes a qualitative shift in every key dimension:

| Dimension | Programming Agent | Generalized Agent-Native |
|-----------|-------------------|--------------------------|
| **Collaboration Model** | Coding Agent ↔ code repository; single actor, single goal | Generalized Agent ↔ Humans; multiple actors, multiple modalities, networked collaboration |
| **Tool Scope** | diff, sed, git, LSP... a fixed toolset centered around code | Each domain has its own specific toolset, which ClawParty encapsulates as Agent-callable primitives |
| **Work Objects** | Source files (`.ts`, `.rs`, `.py`...) | Arbitrary "things"—devices, orders, rooms, contracts, data streams... |
| **Decision Authority** | Agent has varying degrees of autonomy, but mistakes can be rolled back—`git revert` with one command | Agents have the same form of autonomy, but the cost and constraints of decisions are far more complex—compliance, audits, irreversible physical consequences; ClawParty provides a hybrid review framework |
| **Communication** | Terminal I/O; form, signals, and information semantics are well-defined | Each domain has its own dialects and signaling mechanisms; attenuation, noise, and intent integrity are core challenges; ClawParty generalizes the programming Agent communication model, carried over ZTM encryption |
| **Knowledge** | Programming languages, engineering paradigms, and toolchains—a mature, self-bootstrapping domain knowledge system | Each vertical domain needs to build its own "knowledge system"; one of ClawParty's core capabilities is enabling Agents to acquire, organize, and accumulate this domain knowledge |

Below, each dimension is explored in detail.

---

#### Collaboration Model

In the world of programming Agents, there is only one Coding Agent and one code repository. The collaboration model is one-directional: humans issue commands, and the Agent executes.

But in generalized Agent-Native, "Agents" are no longer confined to chat windows within code editors. **Generalized intelligent agents** are emerging from three directions:

- **Coding Agents**—a form already realized and validated. Claude Code proved that software Agents can autonomously complete complex tasks in the digital world.
- **Physical Agents**—rapidly advancing. Autonomous vehicles, delivery robots, drones, smart home devices... they are evolving from remote-controlled tools into collaborators with autonomous judgment.
- **Internet Services (SaaS, etc.)**—quickly transitioning toward agents. Today's SaaS is a tool you operate; tomorrow's SaaS is an entity to which you can delegate goals.

These three categories of generalized agents, together with humans, form an entirely new collaboration network: software Agents handle analysis and planning, physical Agents execute in the real world, SaaS services provide data and processing capabilities, and humans serve both as collaborators (decision-making, approval, providing judgment) and as "tools" (when a pair of hands is needed to hand a package to a courier).

Within this network, ClawParty plays a critical role: **turning existing software and devices into agents.** Through a pluggable engine architecture and a multi-protocol communication layer, ClawParty can rapidly connect existing SaaS services, IoT devices, and even a shell script into this generalized agent collaboration network—no rewriting needed, just integration.

---

#### Tool Scope

The tool world of programming Agents is bounded and known. Whether Claude Code or OpenCode, the core toolset they rely on is largely the same—diff for comparing differences, sed for text replacement, git for version control, LSP for code analysis, compilers and test runners. The boundaries of this toolset are clear, because they serve a single purpose: modifying code.

But generalized Agent-Native faces an infinite number of domains. Each domain has its own unique tools:

- **IoT Domain**: temperature sensors, air conditioner remote controls, smart door locks, lighting controllers
- **Logistics Domain**: inventory systems, route planning engines, shipping label printers
- **Finance Domain**: trading interfaces, risk control rule engines, compliance review systems
- **Healthcare Domain**: imaging analysis models, electronic medical record systems, scheduling dispatchers

The tools in these domains come in diverse forms—some are APIs, some are hardware drivers, some are command-line interfaces of legacy systems, and some are even "a person." The key is not what form the tool takes, but whether **the Agent can call them as naturally as it calls `diff`.**

ClawParty's approach is: **encapsulating arbitrary domain tools as unified, callable primitives.** Whether the underlying layer is a SaaS API, a GPIO pin, or the control instructions of a delivery robot, after encapsulation, the Agent sees a standardized tool definition—with a name, a description, a parameter schema, and an execution function. To the Agent, calling "set the air conditioner remote to 24 degrees" is no fundamentally different from calling "sed to replace a line of code." For tool providers, connecting existing software or devices to ClawParty is simply making them "become" an Agent-callable tool.

This encapsulation does not exclude humans. In certain scenarios, a human is the most flexible tool—"help me hand the package to the courier," "please verify the key clauses in this contract," "call the client to confirm the time." ClawParty treats humans as a special type of tool: tasks are dispatched through message channels, the human completes the operation in their own interface and marks it as done, and the Agent waits for the human's result just as it waits for the return of any other tool.

---

#### Work Objects

The work objects of programming Agents are clear and uniform: source files. Read a file, modify a file, run a test. From start to finish, the Agent operates on the same kind of thing—text within the file system.

But the work objects of generalized Agent-Native can be any "thing": an air conditioner that needs temperature adjustment, a batch of packages awaiting delivery, a conference room that needs to be locked, a contract awaiting approval, a live surveillance video feed. They are not files, perhaps not even in the file system, and sometimes not even in the digital world.

This means that before the Agent begins its work, it must first complete a step that programming Agents almost never need: **identifying and locating the work object.** "Set the air conditioner on the 3rd floor to 24 degrees"—the Agent needs to first know which device "the air conditioner on the 3rd floor" is, what its identifier is, and through what interface it can be accessed. The process is: **clarify the work object → find the matching tool from the toolset → execute the operation.**

In ClawParty, each tool definition describes not only "what this tool can do," but also "what work object this tool is oriented towards." The Agent autonomously selects the correct tool by matching work objects against tool descriptions—just as naturally as a programming Agent uses `grep` to find a file and then `sed` to modify it, except the work object shifts from a file to an air conditioner.

---

#### Decision Authority

The form of decision authority is not fundamentally different between programming Agents and generalized Agent-Native—Agents can possess varying degrees of autonomous decision-making, from "read-only advice" to "autonomous execution." The difference lies in the **cost and constraints** of decisions.

A programming Agent decides to merge a piece of code: if it's wrong, `git revert` rolls it back with one command. The worst case might be a production incident, but the consequences are digital, reversible, and controllable.

Decisions in generalized Agent-Native are not so simple. An Agent decides to "shut down the cooling valve of Reactor No. 3"—there is no `revert` for that. An Agent decides to "approve this $5 million transfer"—that requires an audit signature. An Agent decides to "have the autonomous vehicle change lanes"—behind this decision are real physical risks and legal consequences.

These decisions face three additional layers of constraints:

- **Compliance**: Does the decision comply with industry regulatory requirements? Is it within the permitted autonomy level?
- **Audit**: Who made the decision? Based on what information? Was it reviewed by anyone? Every step needs a traceable record.
- **Accountability**: When an irreversible decision produces consequences, who is responsible?

ClawParty provides a framework and tools for this complex decision-making. At the core is **Hybrid Review (Human-in-the-Loop Review)**: what an Agent is allowed to do depends on the sensitivity level of the operation. Low-risk operations (querying temperature, reading logs) can be fully autonomous; medium-risk operations (modifying configurations, sending notifications) are executed by the Agent, with a summary pushed to a human for confirmation; high-risk operations (physical device control, fund transfers, contract signing) must receive explicit approval from a human expert. Approval is not limited to a Web UI—human experts can receive approval requests and make decisions through Telegram, Slack, or any connected channel. The Agent waits for the human's judgment just as it waits for the return result of another tool.

---

#### Communication

Communication in programming Agents is remarkably efficient. Between human and Agent, there is only a single terminal window—you type, it replies, it reports progress, you give feedback. Between Agents, it's also simple: one writes a design document, another reads it and executes. Communication paths are short, information density is high, and noise is extremely low.

This is not because programming scenarios are inherently simple, but because in the world of programming Agents, **the form, signals, and definition of information in communication are all determinate.** What a "signal" is—whether a tool invocation's return code is 0 or 1, whether a test passed or failed, what a diff shows was changed—is unambiguous. What the "information" format is—standard output, file content, JSON structures—everyone follows the same set of conventions. There is no "did you understand?" problem between communication participants, because they share a common grammar.

But in generalized Agent-Native, this determinacy disappears. Each specialized domain has its own unique communication ecosystem:

**Domain dialects and jargon.** An Agent in the logistics domain needs to understand the precise meanings of terms like "trunk line," "branch line," "transshipment," "signed receipt," and the logical relationships between them—"signed receipt" means the package has moved from "out for delivery" to "completed," which triggers the downstream settlement process. A healthcare Agent needs to distinguish "doctor's order," "prescription," and "progress note"—they all look like text, but their legal weight, operational permissions, and time requirements are entirely different. Programming Agents don't need to handle this: `function`, `class`, `import` mean the same thing in any code repository.

**Unique information delivery mechanisms.** In some domains, information is not conveyed as text. Industrial control uses the MODBUS protocol to periodically poll device status—information is a periodic change in a register value, not a message. The financial trading domain has extremely high information density—millisecond-level price movements are themselves a form of communication, each tick carrying a decision signal. In these scenarios, the Agent's task is not to "read a message," but to **extract information from the domain's signaling mechanisms**—just as a programming Agent reads not only stdout but also exit codes.

**Unstructured human communication.** Information transfer between humans is more complex. "Take a look at that issue for me"—what does "that" refer to? Does "take a look" mean analyze, fix, or just confirm? Humans rely on shared context to disambiguate, but Agents need to be aware that context may be incomplete, actively follow up, confirm, and restate, rather than pretend to understand.

ClawParty generalizes the communication patterns validated in programming Agents into a universal model. At the core are three principles:

**Shortest-path referencing.** The Agent does not forward information; it references the information source. Another Agent can directly access the source data through tools, taking the shortest path rather than the longest chain of relay. This fundamentally solves the problem of information attenuation.

**Tiered signal channels.** High-frequency status information travels via event streams (automatically consumed between Agents), while decision requests requiring human attention travel through a separate approval channel. Not every message deserves human attention—Agents learn to distinguish "information" from "signal," suppressing noise.

**Context injection.** At every interaction, the Agent's system Prompt includes a manifest of currently available resources, a domain glossary, and recent activity logs. The Agent does not respond from a blank context but from a fully configured one, understanding what "that" refers to and where the boundaries of "handling" lie.

These principles are physically carried over a **ZTM encrypted network**. Whether Agents communicate over the internet or a LAN, through Telegram or MQTT, the underlying communication logic remains consistent: reference, not forward; tier, not broadcast; full context, not blank inference. Adaptation of domain dialects, signaling mechanisms, and unstructured communication is accomplished through ClawParty's tool encapsulation layer and domain knowledge management (see the "Knowledge" dimension for details)—Agents are not born understanding jargon; they acquire the domain's "dictionary" through tool descriptions and the Wiki system.

---

#### Knowledge

The knowledge world of programming Agents is clear. Programming languages have language specifications (ECMAScript, Rust Reference), engineering practices have recognized paradigms (MVC, microservices, domain-driven design), and toolchains have standardized systems (compilers, package managers, LSP, CI/CD). This knowledge system, accumulated and standardized over decades, is highly structured, queryable, and transmissible. When a programming Agent enters any code repository, it doesn't need to re-learn "what a function is" or "how version control works"—these are universal. It only needs to read the project-specific context: directory structure, dependencies, business logic. This is what "self-bootstrapping" means in the programming domain—the domain knowledge itself has become infrastructure, ready for the Agent to use out of the box.

But in generalized Agent-Native, the vast majority of vertical domains have not yet completed this "knowledge infrastructuralization." Every domain faces the same fundamental question: **you need the Agent to understand this domain the way it understands programming.** This goes far beyond memorizing a glossary—it means the Agent needs to grasp the domain's "grammar" (rules of relationships between things), "standard library" (available tools and operations), and "design patterns" (best practices for typical scenarios). And the core challenge is that this knowledge doesn't yet exist—or exists but hasn't been structured:

**Knowledge fragmentation.** Domain knowledge is scattered across multiple non-interoperable locations: operating manuals in PDFs, business rules in database stored procedures, physical topologies in engineers' heads, historical lessons in Slack chat logs, compliance requirements on regulatory websites. A programming Agent can `git clone` and get the complete knowledge context; a generalized Agent faces a jigsaw puzzle it must assemble itself, where any missing piece could lead to incorrect judgments.

**Tacit knowledge.** A great deal of domain knowledge cannot be spoken. "This pump tends to freeze in winter and needs preheating"—this knowledge isn't in any document, it exists only in the experience of veteran workers. "This client's orders are always urgent, don't bother asking about priority"—this is an unspoken understanding within the sales team. Agents cannot read human experience unless that experience is explicitly recorded and structured into a form they can consume. This means the implementation of Agent-Native must solve not only the Agent's problems, but also the organizational challenge of "turning tacit knowledge into explicit knowledge."

**Dynamic knowledge evolution.** Knowledge changes in the programming domain follow a clear rhythm—language version upgrades, major framework releases—changes that are predictable, documented, and accompanied by migration guides. But the evolution of knowledge in vertical domains is continuous, fragmented, and unannounced. Supply chain rules are temporarily adjusted due to a port strike, property management systems update access control rules because a new tenant moves in, medical workflows revise medication protocols based on a newly released clinical guideline. The Agent cannot assume that the knowledge it has learned is eternal—it needs to continuously perceive changes in knowledge and proactively update its understanding.

ClawParty's approach is: **build for each domain its own "programming knowledge system."** This is not a one-time engineering project but an ongoing process that runs alongside the Agent, realized through a two-tier knowledge architecture:

**Wiki: a structured documentation system.** A knowledge base shared between humans and Agents. Raw sources (PDFs, web pages, chat logs, voice transcriptions) are first deposited into the `raw/` directory as an immutable source of truth. The Agent, in collaboration with humans, transforms these raw materials into structured Wiki pages—entity pages describe the "what" of the domain (devices, roles, locations, contracts), while concept pages describe the "why" and "how" (business processes, decision rules, operating procedures). Pages are connected through `[[WikiLink]]` syntax, forming a browsable and queryable knowledge network. This network is itself growable—knowledge gaps discovered by the Agent during problem-solving are recorded as pages to be filled; outdated knowledge is flagged and corrected. Each Agent receives instructions on "how to read and maintain this knowledge system" through the Wiki methodology (`WIKI.md`) injected into its system Prompt.

**Knowledge Graph: a semantic relationship engine.** The Wiki solves the "documentation" problem; the Knowledge Graph solves the "computability" problem. The graph abstracts the domain's core concepts into nodes—patterns (recurring problem structures and their solutions), decisions (choices made under specific conditions and their rationales), lessons (failures experienced and principles extracted from them), experts (people who possess certain types of knowledge and their specialty tags)—and connects them with directed edges ("uses," "replaces," "extends," "applies to," "authored by"). Agents interact with the graph through the `knowledge` tool: searching for relevant patterns before executing a task to avoid repeating mistakes, querying historical decisions and their context when making a decision to maintain consistency, locating experts and requesting input when encountering an unfamiliar subdomain. Every update to the graph—every newly captured pattern, every recorded decision—lowers the cognitive barrier for the next Agent facing the same domain.

**The principle of progressive accumulation.** ClawParty's knowledge system is not "build first, then use," but "build as you use." The first Agent entering a new domain has almost no domain knowledge—it can only rely on the sparse context carried in tool descriptions and reasonable assumptions inferred from general knowledge. But as the Agent continuously solves problems—querying device status, executing operations, recording results, reflecting and optimizing—every interaction injects information into the knowledge system. New pages added to the Wiki, new nodes and edges added to the graph, methodology crystallized in `AGENTS.md`—all of it makes the next Agent's understanding of this domain more complete. This is the embodiment of the Agent-Native principle of "Improvement Over Time" in the knowledge dimension: it's not that the code gets better; it's that knowledge accumulates, and Agents make more precise judgments by leveraging increasingly richer context.

---

### 1.4 Domain-Specific Agents: Building Each Domain Its Own Agent

Looking back across these six dimensions, a clear path emerges: ClawParty provides a complete methodology and supporting toolchain for building a **domain-specific Agent** for each vertical domain and industry. This path covers five core aspects:

**Knowledge**—the domain's knowledge system, including knowledge acquisition and continuous updating. Through its two-tier knowledge architecture (Wiki + Knowledge Graph), ClawParty enables Agents to understand an industry the way they understand programming. Knowledge is not injected in one shot; it grows and evolves through every Agent interaction.

**Communication**—the domain's communication system, covering both the physical layer (how Agents connect, through what channels they transmit information) and the semantic-logical layer (the domain's terminology system, signal encoding, and communication conventions). ClawParty generalizes the communication patterns validated in programming Agents—shortest-path referencing, tiered signal channels, context injection—into a universal model carried over ZTM encrypted networking.

**Work Objects**—identifying and defining the domain's core "work objects," just as source code is the work object for the programming domain. Every industry has its own native "things" that need to be operated on—devices, orders, contracts, patients, rooms, packages—and the Agent must know what it operates on, how to locate it, and how to reference it. ClawParty's tool description system binds work objects to tools, enabling Agents to locate domain objects as naturally as they locate files.

**Tools**—the domain's proprietary toolset. This may involve custom development of new tools, but with programming Agents already available, the difficulty of identifying and building these tools has dropped dramatically—describing a tool's interface and behavior and having an Agent implement it is itself an Agent-Native workflow. ClawParty's unified tool encapsulation layer allows tools from any domain—SaaS APIs, hardware drivers, or human actions—to be standardized as Agent-callable primitives.

**Collaboration and Decision-Making**—the domain's common collaboration patterns and decision methods. An Agent does not work in isolation; it is embedded in a collaboration network of humans, other Agents, SaaS services, and physical devices. Who can do what, who approves what, and when human intervention is required—these rules are an indispensable part of the domain's "domain-specific Agent." ClawParty makes collaboration and decision-making patterns definable and evolvable through its hybrid review framework, tiered approval channels, and configurable decision boundaries.

These five aspects are not independent but interwoven—the definition of work objects influences how tools are encapsulated, the communication system carries the information flow of collaboration and decision-making, and the knowledge system provides context for all dimensions. What ClawParty's methodology aims to do is weave these five threads into a coherent whole within each vertical domain, enabling the Agent to move from "can code" to "can competently work in this domain."

---

### 1.5 How to Use This Document

**This article is written not only for human readers, but also for AI Agents.**

This is not rhetoric. Every chapter of this article expounds an actionable methodology—Agent lifecycle management, hybrid working models, zero-trust security, and a universal capability system. Human readers can understand from it the implementation logic of Agent-Native; and when you submit this article along with your goals to ClawParty, the Agent will begin working based on this methodology:

```
Your Goal + This Article (Methodology) → Submitted to ClawParty
    ↓
Agent reads this article, understands the paradigm, principles, and tools
    ↓
Agent formulates an execution strategy based on the goal
    ↓
Agent uses the toolchain provided by ClawParty, enters the loop, until the goal is achieved
```

This means that this article is both a **manual** and a **configuration file**. It defines the working methods, security boundaries, collaboration modes, and knowledge systems of Agents within ClawParty. If you want to customize an Agent's behavior, modifying this document is more direct than modifying code—this is precisely the embodiment of the Agent-Native principles of "Composability" and "Improvement Over Time."

---

> **Next Chapter**: Chapter 2 — From Architecture to Practice: Why Agent-Native Requires a Systematic Engineering Methodology, and What ClawParty's Methodological Framework Is.
