# Viv Script

## TODO
- [ ] Write a parser from scratch to replace `nom`
    * will provide better error reporting 
- [ ] Create a general system for type passing without special casing everything!
- [ ] Make strings hold their length in a struct (i.e smart pointers from rust)
- [ ] create function definition and move to having a `main`
- [ ] Allow importing files
    - [ ] gather type information from files
        - [ ] How to handle circular imports? 
            * option 1: dont allow them
                * It is usually bad design, so lets not? 

## BIG_TODO
WRITE FUCKING TESTS!
* we should wait until we got a main, so we wont screw over all the tests once we switch to that.

## Vars

* No pointers exposes to the user 
* Every variable owns its data.
    * If a borrowed value is passed to a assignemnt we memcpy it.
* Arguments are passed by reference (avoids memcpy if no operations needing owned values are used)