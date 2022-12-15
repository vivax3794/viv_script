# Viv Script

## TODO
- [x] Write a parser from scratch to replace `nom`
    * will provide better error reporting 
- [x] Create a general system for type passing without special casing everything!
- [x] create function definition and move to having a `main`
- [ ] Allow importing files
    - [ ] gather type information from files
        - [ ] How to handle circular imports? 
            * option 1: dont allow them
                * It is usually bad design, so lets not? 

## Vars

* No pointers exposes to the user 
* Every variable owns its data.
    * If a borrowed value is passed to a assignemnt we memcpy it.
* Arguments are passed by reference (avoids memcpy if no operations needing owned values are used)