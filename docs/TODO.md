# TODO

* I’m starting to think about how to generate some kind of tiled or chunked terrain/environment where some chunks need to be generated during setup and then as you move around in the world, more are generated (first time)/loaded (second time and after) and those that are out of range are unloaded.
At a very basic level, it seems to me like each chunk would be an asset that is addressable in some consistent way, probably based on some kind of chunk coordinates.
This implies both dynamic generation of assets and dynamic loading of assets. I understand the asset server can handle dynamic loading. I suppose one would add a layer on top of that that identifies whether an asset needs to be generated, persisted or unloaded. That would probably be a resource. Then one could have a system that uses that resource and player coordinates to carry out the work
Does this make sense?
Or else how would be good options to do this?
It’s probably good if the chunk generation can be done asynchronously and in parallel.
I suppose one has to minimise the time spent in systems that take mutable resources as parameters. So they should make use of async I/O and do something sensible when it comes to any use of the resource in the next game loop iteration. Seems like some interesting problems to solve

If you have time, I’d love to hear your thoughts on my question about how to implement a chunked environment/tiled heightmap in bevy where chunks are generated on the fly  the first time they are visited and persisted to disk, and then loaded from disk if they already exist. As well as loading and unloading chunks so that you have only those within a certain distance from the player loaded. I brainstormed an idea in the backlog but I’m concerned that using resources will cause bottlenecks

Thanks. It’s really appreciated. I wasn’t sure if you were only responding to ones that looked unanswered, hence the request. :slight_smile: I feel like I need to wrap my head around the patterns of leveraging an ECS and I feel like I’m missing something with resources, I see a risk that I’ll bundle too many things together and kill parallelisation of systems
cartYesterday at 9:27 PM
as the character moves around, you will want to load (and / or generate) them when the character is "close". then when the character "leaves" or enough time has passed, save them to disk.
robswainYesterday at 9:27 PM
So I’m kind of also thinking that the chunks that get loaded in become entities in some way or other. But using some async functionality that doesn’t block the game loop seems like a way around that
cartYesterday at 9:27 PM
asset loading is already async, so it wont block when you load from disk
(unless you use the []_sync methods)
robswainYesterday at 9:28 PM
Yup. I get the concept of what I’m trying to do. It’s more how to do it well within an ECS with systems and resources, not blocking the game loop nor bottlenecking it due to some mut Resource in a system
That’s good. So then generation needs to be async too unless it can be made fast enough
cartYesterday at 9:29 PM
yeah generation is likely an expensive operation. you can queue up work on bevy's task system if you don't want it to block
robswainYesterday at 9:30 PM
It’s possible I’m overthinking and should just do it and see what happens then optimise when I hit problems. Usually a good strategy, but I wanted to get some input from people who know better before I put too much time into it
Oh there’s something like a job queue?
cartYesterday at 9:30 PM
yupyup. check out bevy_tasks. we currently use them to execute the bevy_ecs schedule, but you can use them in any context
however for simplicity i would probably start by letting the asset loading block
robswainYesterday at 9:31 PM
Cool. So then I’d queue up generation as a task, and then I guess I’d need a way of handling completed tasks?
cartYesterday at 9:31 PM
* the async work block
then move it to a separate thread when you've verified that it works
yeah i would probably use a Channel
robswainYesterday at 9:32 PM
A system that handles the completed generation tasks, loads them into the scene, and async stores them to disk

if you're worried about a specific resource type being used everywhere and you dont want it to block systems, you can add some interior mutability for common operations so you can use Res<T> instead of ResMut<T>
robswainYesterday at 9:33 PM
I’ve used rust channels, are bevy Channels similar?
cartYesterday at 9:33 PM
but in general id only do that when it becomes a real, measurable problem
robswainYesterday at 9:33 PM
Mmm. Ok

n this case i dont think many people have used bevy_tasks for queuing up async work, so we don't have any good examples (or established patterns) yet. i think you might want to use the crossbeam_channel crate

Ok. I’ve seen it used by other crates but haven’t used it myself
cartYesterday at 9:37 PM
its pretty straightforward. you create a new Channel<T>, which has a Sender<T> and Receiver<T>. You pass the sender to your async task and you use the receiver in your system to listen for "finished" results
sender and receiver are both cheaply cloneable
so you can have multiple senders or multiple receivers
robswainYesterday at 9:38 PM
Same as rust mpsc then. But mpmc
Cool

Are there asset streaming examples? I guess this isn’t too different from hotloading. I ran that example but it didn’t seem to do anything. It just showed the monkey head last I checked
Anyway, thanks for all the input. I’ll roll with this and see how I get along :slight_smile:

Any tips for good 2D/3D/4D perlin/simplex noise crates?

@robswain nothing specifically dedicated to asset streaming. its also worth pointing out that we dont really stream assets in bevy. we load them in complete chunks asynchronously on separate threads.

i dont have any noise crate recommendations. havent used any of the options yet
in general just generate the "new" assets on a separate thread. and load() them when you're ready
robswainYesterday at 9:42 PM
simdnoise, bracket-noise look actively developed and commonly used
Sweet :slight_smile:
cartYesterday at 9:42 PM
and you can listen for AssetEvent<T> if you want to detect/respond to changes to assets on disk (hot-reloading)
