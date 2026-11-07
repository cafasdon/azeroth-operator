//go:build with_db

// Build this adapter from the pinned WoWSims Forever Go module.
package main

import (
	"errors"
	"fmt"
	"io"
	"os"

	"github.com/wowsims/forever/sim"
	"github.com/wowsims/forever/sim/core/bulk"
	"github.com/wowsims/forever/sim/core/proto"
	"google.golang.org/protobuf/encoding/protojson"
)

const maxRequestBytes = 8 << 20

func run(input io.Reader, output io.Writer) error {
	raw, err := io.ReadAll(io.LimitReader(input, maxRequestBytes+1))
	if err != nil || len(raw) > maxRequestBytes {
		return errors.New("could not read bounded bulk request")
	}
	request := &proto.BulkSimRequest{}
	if err := protojson.Unmarshal(raw, request); err != nil {
		return errors.New("invalid bulk request JSON")
	}
	if request.BaseRequest == nil || request.BulkSettings != nil || request.AllResults ||
		len(request.Candidates) == 0 || len(request.Candidates) > 64 ||
		request.TopResults < 1 || int(request.TopResults) > len(request.Candidates) {
		return errors.New("expected a baseline, explicit candidates and bounded shortlist")
	}
	seen := make(map[int32]bool, len(request.Candidates))
	for _, candidate := range request.Candidates {
		if candidate.GetGear() == nil || candidate.Index < 2 || seen[candidate.Index] {
			return errors.New("expected distinct candidate indices above one and complete gear inputs")
		}
		seen[candidate.Index] = true
	}

	sim.RegisterAll()
	progress := make(chan *proto.ProgressMetrics, 32)
	bulk.BulkSimAsync(request, progress, "local-forever-bulk")
	var result *proto.BulkSimResult
	for update := range progress {
		if update.FinalBulkSimResult != nil {
			result = update.FinalBulkSimResult
		}
	}
	if result == nil || result.Error != nil || result.Baseline == nil || len(result.TopResults) == 0 {
		return errors.New("bulk simulation failed")
	}
	for _, row := range append([]*proto.BulkGearResult{result.Baseline}, result.TopResults...) {
		if row.GetDpsMetrics() == nil {
			return errors.New("bulk simulation returned incomplete metrics")
		}
		row.Gear = nil
		row.DpsMetrics.AllValues = nil
	}
	encoded, err := (protojson.MarshalOptions{EmitDefaultValues: true}).Marshal(result)
	if err != nil {
		return errors.New("could not encode bulk result")
	}
	_, err = output.Write(encoded)
	return err
}

func main() {
	if err := run(os.Stdin, os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
