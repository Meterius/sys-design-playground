export enum OsmType {
    Node = 'node',
    Way = 'way',
    Relation = 'relation',
}

export interface OsmId {
    type: OsmType,
    key: number,
}

export function extractOsmIdFromOmtFeatureId(featureId: number): OsmId | null {
    const key = Math.floor(featureId / 10);

    switch (featureId % 10) {
        case 0: return { type: OsmType.Node, key };
        case 1: return { type: OsmType.Way, key };
        case 4: return { type: OsmType.Relation, key };
        default: return null;
    }
}