# g-flite: step-by-step

In this tutorial, we will build a minimalistic version of [g-flite] app,
fully functional, albeit devoid of unnecessary features such as animated
progressbar.

The idea behind the app is very simple: we want to run the [flite] text-to-speech
program, compiled to Wasm (currently, gWasm requires apps to target `wasm32-unknown-emscripten`
target), distributed across the Golem Network. Thus, given a text input, our app
needs to:
1. package the Wasm binary
2. split the input into some nonoverlapping chunks of text for processing with each
  chunk processed in parallel by some Golem node
3. prepare a gWasm task using `gwasm-api`, and send it to our Golem instance for
  distributing across the Golem Network
4. and finally, combine the resultant WAV chunks into one final WAV which represents
  our input text converted to speech

The step 3. described above is the one where we will interface with `gwasm-api` crate,
and this is the step where we can orchestrate progress updates to the user.

[g-flite]: https://github.com/golemfactory/g-flite
[flite]: http://www.festvox.org/flite/
