# DESIGN

## What is minkraft?

A minecraft-like voxel world with chunked environment that is generated, persists, is modifiable, has various block types that are mineable / placeable, has blocks that have their own physical systems (water/lava, fire, plants that grow).

## Environment

### Coordinates

* World to chunk to block coordinates
  * 16x16x16 blocks in a chunk

### Generation

#### Global Definitions

* Bedrock, sea, maximum height levels
  * 0 is bedrock
  * 64 is sea level
  * 255 is maximum height of the world above the bedrock

#### Input Parameters

* Provide a seed to select a world
* Chunk/block coordinates in the world?
* **NOTE**: Everything must be generated in a consistent way based on the world seed and world coordinates in a deterministic way

It is infeasible to always generate the world in a perfectly consistent way if using random number generators seeded with the world seed. This is because we would always have to generate the blocks in the exact same order, and we may want to start with nothing generated, spawn a player at a random x,z location, and then generate the chunks around them that are necessary to identify where exactly to place the player. Nothing exists until it is visited.

We will need to consider the scope of space over which to seed the generation. For example, the biomes, surface, caves, and anything that can span multiple chunks need to be globally sensible and so must be sampled from a large space effectively seeded _once_ by the world seed.

For generation of local things like object placement we must use a smaller unit of space and ensure that that is always generated deterministically. This could be a biome seed, or a chunk seed. A biome seed could be based on the world seed and the coordinates of the chunk stack (stacking in y) with least x,z. A chunk seed is a combination of the world seed and its x,y,z coordinates.

It seems feasible that smaller scopes can use values sampled from larger scopes - for example, a the surface within a biome may be affected by what biome it is in. Or local placement of objects may be affected by the biome and the surface.

#### Steps

* Generate biomes
  * Defines default block and block palette
* Generate a surface
* Generate underground blocks
* Generate caves
* Place objects

### Prototyping Tasks

* Generate biomes
  * Just visualise generating cellular/some kind of bacterial noise?
* Generate the surface within those biomes

### Storage

#### In-Memory Format

#### Maintenance

#### On-Disk Format
