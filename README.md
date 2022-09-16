# mcscan-rs

A simple Minecraft server scanner that outputs info on servers found via a Masscan output.

# Usage

`mcscan-rs -i <INPUT>`

Outputs servers as a CSV, but the CSV is dynamic and doesn't have fields so you can't really feed it into other programs.

Note that the program outputs to stdout. To save the output to a file, pipe the output like so:

`mcscan-rs --input <INPUT> > out.csv`

# Performance

It's decently fast. Most of the time here was spent waiting for the last few to time out.

`Processed 3319 servers in 00:02:30 (2037 failed)`