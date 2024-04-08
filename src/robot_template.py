def surrounding_tiles(tile):
    return (
        Coords(tile.x, tile.y+1),
        Coords(tile.x, tile.y-1),
        Coords(tile.x-1, tile.y),
        Coords(tile.x+1, tile.y)
              )
    
def corner_tiles(tile):
    return (
        Coords(tile.x+1, tile.y+1),
        Coords(tile.x+1, tile.y-1),
        Coords(tile.x-1, tile.y+1),
        Coords(tile.x-1, tile.y-1)
    )
    
def allies_around_corner(tile, state):
    return sum(is_friendly(edge, state) for edge in corner_tiles(tile))
    
def unsafe_surrounding_tiles(tile, state):
    return sum(is_enemy(edge, state) for edge in surrounding_tiles(tile))

def friendly_surrounding_tiles(tile, state):
    return sum(is_friendly(edge, state) for edge in surrounding_tiles(tile))

def empty_surrounding_tiles(tile, state):
    return sum(is_terrain(edge, state) for edge in surrounding_tiles(tile))

def is_friendly(tile, state):
    return (obj:=state.obj_by_coords(tile)) is not None and obj.team == state.our_team

def is_enemy(tile, state):
    return (obj:=state.obj_by_coords(tile)) is not None and obj.team == state.other_team

def is_terrain(tile, state):
    return (obj:=state.obj_by_coords(tile)) is not None and obj.obj_type == ObjType.Terrain

def robot(state, unit):
    enemies = state.objs_by_team(state.other_team)
    closest_enemy = min(enemies,
        key=lambda e: (
            e.coords.walking_distance_to(unit.coords),
            -allies_around_corner(e.coords, state)-friendly_surrounding_tiles(e.coords,state),
            e.health
        )
    )
    closest_ally = min((i for i in state.objs_by_team(unit.team) if i.id != unit.id),
        key=lambda e: (e.coords.walking_distance_to(unit.coords), e.health)
    )
    direction_to_center = clockwise_direction_towards(unit.coords, Coords(9,9))

    move = {}
    return move