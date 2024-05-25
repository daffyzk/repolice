# repo-hub

---

## What is repo-hub?

The idea behind repohub is to have a CLU that allows you to list the current status of all the repositories that are stored under a specific directory.

Lets say you have a directory named '/git' and said dir, holds multiple repositories (could be personal projects, work repos, etc.).

With repo-hub you can run 'repos' from said directory or you can pick any directory and run 'repos /home/git/work/'.
This returns a list of the status of all repos within that directory. With the amount of new files, file changes, etc. in a minimal format.

You can also specify an expressive return with the '-x' option.
You can also specify the directory depth of searches with the '-d [int]' option. 
There is a '-h' option that lists all options and explains the functionality.
