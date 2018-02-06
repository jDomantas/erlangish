# Erlangish

A stupid toy programming language. Basically the only operations you can do are:

* Spawn an actor,
* Receive a message,
* Send a message to another actor.

Why "erlangish"? Erlang has actors. This language has actors. Ta-da. Don't judge
me, naming is hard and I only spent an evening on making this.

## Installing

If you have rust and cargo installed, you can build this from source. This
requires nightly toolchain, because of that delicious `#![feature(nll)]`.

```
git clone https://github.com/jDomantas/erlangish.git
cd erlangish
cargo run -- repl
```

If you don't want to build this from source and are on windows, there's a
[windows binary attached to a github release](https://github.com/jDomantas/erlangish/releases/tag/v0.1.0).

## Language

Grammar, in some probably understandable notation:

```
<expr>      ::= <symbol>
              | <variable>
              | "root"
              | "receive"
              | "{" <statement>* "}"
              | "spawn" <expr>

<statement> ::= <expr> ";"
              | <expr> "!" <expr> ";"
              | "let" <variable> "=" <expr> ";"

<variable>  ::= [a-zA-Z_][a-zA-Z0-9_]*
<symbol>    ::= '[a-zA-Z_][a-zA-Z0-9_]*
```

Expression evaluation rules:
* Symbol evaluates to itself. The only reason to have symbols is that they are
displayed nicely.
* Variables evaluate to the value they were previously bound to. Evaluating
unbound variable is an error.
* `root` evaluates to the handle of root actor. Messages sent to `root` will
be printed to stdout.
* `receive` will wait until there's at least one message in current actor's
message queue, and then pop the first message and return it.
* Blocks evaluate to, well, blocks. They capture parent scope, so inside them
you can refer to values bound in parent block.
* `spawn <expr>` evaluates `<expr>` and creates a new actor. `spawn` argument
must evaluate to a block. `spawn` expression itself evaluates to an actor handle
of the just spawned actor, to that you can send messages to it. Spawned actor
will evaluate given block once, and then die.

Statement evaluation rules:
* Expression statements simply evaluates that expression and discard the result.
* `<actor> ! <message>` will evaluate `<actor>` and then `<message>`, and push
the message in the actor's message queue. `<actor>` must evaluate to an actor
handle.
* `let <var> = <expr>` will evaluate `<expr>` and bind the name to that value.

If you run the repl, you can enter statements one by one. When you enter a
statement, the root actor will first evaluate it, then wait until all actors are
idle, and then print roots message queue to stdout (and also clear the queue).

Running a file behaves somewhat the same - it behaves as if all the statements
in the file are entered into the repl one by one. Note that this means that root
actor will behave somewhat weirdly with regards to receiving: code
`root ! 'test; receive;` will deadlock, as after the first statement message
queue will be printed and cleared, and then `receive;` will have nothing to
receive.

## Examples

Here are a few examples. You can paste this into a file and
`erlangish run <file>` it, or enter the statements one by one into the repl.

* Prints `Received: 'Hello_world`:
```
root ! 'Hello_world
```
* Sends an infinite amount of `'ping` to root actor (you will only see the
first one, because interpreter waits until everything is idle before printing):
```
let loop = {
    root ! 'ping;
    let loop_code = receive;
    let clone = spawn loop_code;
    clone ! loop_code;
};
let looped = spawn loop;
looped ! loop;
```
* Lexical scope (which I implemented accidentaly)! Prints `Received: 'first'`:
```
spawn {
    let a = 'first;
    let other = spawn {
        receive;
        root ! a;
    };
    let a = 'second;
    other ! 'go;
};
```
* Booleans and ifs!
```
let true = {
    let receiver = receive;
    let value = receive;
    receive;
    receiver ! value;
};
let false = {
    let receiver = receive;
    receive;
    let value = receive;
    receiver ! value;
};
let if = {
    let receiver = receive;
    let bool = spawn receive;
    bool ! receiver;
    bool ! receive;
    bool ! receive;
};
let selector = spawn if;
selector ! root;
selector ! true;
selector ! 'print_this;
selector ! 'dont_print_this;
```
