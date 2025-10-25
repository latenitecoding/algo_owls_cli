# owlgo
The Algo Owls CLI is a lightweight and blazingly fast (**sorry**) tool to provide LLM integration into your programming practice.

![Made with VHS](https://vhs.charm.sh/vhs-ZhGiYcFiYr0DGpvbPJrZI.gif)

The above example was generated with VHS ([view source](https://github.com/charmbracelet/vhs)).

## Tutorial

To get started, [install owlgo](#installation). Fresh installs won't typically come with templates files, so you'll need to first create a program. If you have a program that you would like to use as a template, you can stash the program.

```sh
owlgo stash -T hello.rs
```

If you do have a template file stashed away, you can initialized a new program from that stashed template.

Once you've finished implementing your solution, you can run the solution using owlgo. This makes it much easier to ensure that you're using the same build/run arguments that competitive programming (CP) contests use. It also has the upside of making it easier to use new languages. **owlgo supports nearly two dozen languages including those officially used by ICPC and USACO.**

If your program seems to be working, it's time to start your quest to solve the problem. Many CP problems have test cases published online. owlgo can fetch these test cases for you.

If you don't want to fetch the test cases, owlgo will always fetch them for you once you attempt a quest. A quest in this case is an attempt to pass all provided test cases for a CP problem. owlgo makes it easy to check your solution against the test cases and provides feedback on each test case.

If you're having trouble solving a problem, understanding the problem description, or would just like to explore the problem further, owlgo provides LLM integration right from your terminal.

Note that the above example has been edited to remove the time that claude took to think. Responses typically vary between 10-30s.
